//! The props a component declares: `defineProps` (a runtime object or array, or a type), the
//! `props` option, `defineModel`, and what each prop's declaration says about a default and a
//! type (the reading of eslint-plugin-vue, without type information: a type it cannot resolve in
//! the file is unknown and its props are not checked).

use super::Cx;
use super::js::{self, args, calls, ident, key_name, members, static_name, string};
use core_syntax::{SyntaxNode, SyntaxNodeExt};
use lang_js::generated::kind as j;

pub(super) struct Prop {
    pub name: Option<String>,
    /// The name as the message says it.
    pub label: String,
    pub node: SyntaxNode,
    pub no_default: bool,
    pub no_type: bool,
}

/// A `defineProps` call of `<script setup>`: `outer` is the `withDefaults` call around it, if any,
/// `decl` the declarator that takes the result.
pub(super) struct DefineProps {
    pub call: SyntaxNode,
    pub outer: SyntaxNode,
    pub decl: Option<SyntaxNode>,
    pub props: Vec<Prop>,
}

pub(super) struct DefineModel {
    pub call: SyntaxNode,
    pub decl: Option<SyntaxNode>,
    pub name: String,
    pub name_node: Option<SyntaxNode>,
    /// The node to report when the model declares no type.
    pub no_type: Option<SyntaxNode>,
}

#[derive(Default)]
pub(super) struct Scan {
    pub props: Vec<DefineProps>,
    pub models: Vec<DefineModel>,
    /// The component objects of the options API, with their `props` option read.
    pub objects: Vec<(SyntaxNode, Vec<Prop>)>,
}

/// The macros at the top of `<script setup>` and the component objects the file exports.
pub(super) fn scan(cx: &Cx) -> Scan {
    let mut scan = Scan::default();
    for stmt in cx.root.children() {
        match stmt.kind() {
            j::VARIABLE_DECLARATION => {
                for d in stmt
                    .children()
                    .filter(|d| d.kind() == j::VARIABLE_DECLARATOR)
                {
                    let init = init_of(&d);
                    if let Some(e) = init {
                        macro_in(cx, e, Some(d), &mut scan);
                    }
                }
            }
            j::EXPRESSION_STATEMENT => {
                if let Some(e) = stmt.children().next() {
                    macro_in(cx, e, None, &mut scan);
                }
            }
            j::EXPORT_DEFAULT_DECLARATION => {
                if let Some(obj) = stmt.children().next().and_then(component_object) {
                    let props = options_props(&obj);
                    scan.objects.push((obj, props));
                }
            }
            _ => {}
        }
    }
    scan
}

/// The initializer of a declarator.
pub(super) fn init_of(d: &SyntaxNode) -> Option<SyntaxNode> {
    d.children()
        .skip(1)
        .find(|c| c.kind() != j::TS_TYPE_ANNOTATION)
}

fn macro_in(cx: &Cx, e: SyntaxNode, decl: Option<SyntaxNode>, scan: &mut Scan) {
    if e.kind() != j::CALL_EXPRESSION || !cx.in_setup(&e) {
        return;
    }
    let first = args(&e).next();
    let (call, outer) = if calls(&e, "defineProps") {
        (e.clone(), e)
    } else if calls(&e, "withDefaults")
        && let Some(inner) = first.clone().filter(|a| calls(a, "defineProps"))
    {
        (inner, e)
    } else if calls(&e, "defineModel") {
        let name = first.as_ref().and_then(string);
        let options = if name.is_some() {
            args(&e).nth(1)
        } else {
            first.clone()
        };
        let typed = js::type_arg(&e).is_some()
            || options
                .as_ref()
                .is_some_and(|o| has_type(o).unwrap_or(true));
        scan.models.push(DefineModel {
            name: name.clone().unwrap_or_else(|| "modelValue".into()),
            name_node: first.filter(|_| name.is_some()),
            no_type: (!typed).then(|| options.unwrap_or_else(|| e.clone())),
            call: e,
            decl,
        });
        return;
    } else {
        return;
    };
    let mut found = DefineProps {
        call,
        outer,
        decl,
        props: Vec::new(),
    };
    found.props = macro_props(cx, &found);
    scan.props.push(found);
}

/// The object a component is defined with: `export default {}`, `defineComponent({})`,
/// `Vue.extend({})`.
fn component_object(e: SyntaxNode) -> Option<SyntaxNode> {
    let e = js::unwrap(e);
    if e.kind() == j::OBJECT_EXPRESSION {
        return Some(e);
    }
    let callee = e.children().next()?;
    let defines = ident(&callee)
        .is_some_and(|n| n == "defineComponent" || n == "defineNuxtComponent")
        || callee.kind() == j::STATIC_MEMBER_EXPRESSION
            && callee
                .children()
                .next()
                .and_then(|o| ident(&o))
                .is_some_and(|o| o == "Vue")
            && callee
                .children()
                .nth(1)
                .and_then(|p| js::first(&p))
                .is_some_and(|p| p == "extend");
    if !defines || e.kind() != j::CALL_EXPRESSION {
        return None;
    }
    args(&e)
        .last()
        .map(js::unwrap)
        .filter(|o| o.kind() == j::OBJECT_EXPRESSION)
}

fn options_props(obj: &SyntaxNode) -> Vec<Prop> {
    let mut out = Vec::new();
    if let Some((_, m)) = js::property(obj, "props")
        && let Some(v) = m
            .value
            .filter(|v| matches!(v.kind(), j::OBJECT_EXPRESSION | j::ARRAY_EXPRESSION))
    {
        runtime_props(&v, &Defaults::default(), &mut out);
    }
    out
}

/// What decides that a prop has a default elsewhere than in its declaration.
#[derive(Default)]
struct Defaults {
    with_defaults: bool,
    /// The defaults of `withDefaults` cannot all be known.
    unknown: bool,
    names: Vec<String>,
    destructured: bool,
    assigned: Vec<String>,
}

impl Defaults {
    fn ignores(&self, name: Option<&str>, typed: bool) -> bool {
        let has = |list: &[String]| name.is_some_and(|n| list.iter().any(|x| x == n));
        typed && ((!self.with_defaults && !self.destructured) || self.unknown || has(&self.names))
            || self.destructured && (name.is_none() || has(&self.assigned))
    }
}

/// One entry of an object pattern: the key it reads, the name it binds, and whether it has a default.
pub(super) struct Entry {
    pub key: Option<String>,
    pub binding: Option<String>,
    pub default: bool,
}

pub(super) fn entries(pattern: &SyntaxNode) -> Vec<Entry> {
    let binding = |n: &SyntaxNode| match n.kind() {
        j::ASSIGNMENT_PATTERN => n.children().next().and_then(|b| js::first(&b)),
        j::BINDING_IDENTIFIER => js::first(n),
        _ => None,
    };
    pattern
        .children()
        .filter(|p| p.kind() == j::BINDING_PROPERTY)
        .filter_map(|p| {
            let head = p.children().next()?;
            Some(if head.kind() == j::ASSIGNMENT_PATTERN {
                // `{ a = 1 }`
                Entry {
                    key: head.children().next().and_then(|k| key_name(&k)),
                    binding: head.children().next().and_then(|k| js::first(&k)),
                    default: true,
                }
            } else {
                let value = p.children().nth(1);
                Entry {
                    key: key_name(&head),
                    binding: value.as_ref().map_or_else(|| js::first(&head), binding),
                    default: value.is_some_and(|v| v.kind() == j::ASSIGNMENT_PATTERN),
                }
            })
        })
        .collect()
}

fn defaults(cx: &Cx, m: &DefineProps) -> Defaults {
    let mut d = Defaults {
        with_defaults: m.outer != m.call,
        ..Defaults::default()
    };
    if let Some(pattern) = m.decl.as_ref().and_then(|d| d.children().next())
        && pattern.kind() == j::OBJECT_PATTERN
    {
        d.destructured = true;
        d.assigned = entries(&pattern)
            .into_iter()
            .filter(|e| e.default)
            .filter_map(|e| e.key)
            .collect();
    }
    if d.with_defaults
        && let Some(arg) = args(&m.outer).nth(1)
    {
        d.unknown = !collect_defaults(cx, &arg, &mut d.names, &mut Vec::new());
    }
    d
}

/// The object or array a node is, or the `const` it names; `None` when that cannot be known.
fn object_or_array(cx: &Cx, n: &SyntaxNode, depth: u8) -> Option<SyntaxNode> {
    match n.kind() {
        j::OBJECT_EXPRESSION | j::ARRAY_EXPRESSION => Some(n.clone()),
        j::IDENTIFIER_REFERENCE if depth < 8 => {
            let d = cx.only_decl(&ident(n)?, false)?;
            (d.kind() == j::VARIABLE_DECLARATOR
                && d.parent()?
                    .children_with_tokens()
                    .any(|t| t.as_token().is_some_and(|t| t.text() == "const"))
                && d.children().next()?.kind() == j::BINDING_IDENTIFIER)
                .then(|| init_of(&d))
                .flatten()
                .and_then(|i| object_or_array(cx, &i, depth + 1))
        }
        _ => None,
    }
}

/// The names the object `n` defines, spreads followed; `false` when some cannot be known.
fn collect_defaults(
    cx: &Cx,
    n: &SyntaxNode,
    names: &mut Vec<String>,
    seen: &mut Vec<SyntaxNode>,
) -> bool {
    let Some(obj) = object_or_array(cx, n, 0).filter(|o| o.kind() == j::OBJECT_EXPRESSION) else {
        return false;
    };
    if seen.contains(&obj) {
        return true;
    }
    seen.push(obj.clone());
    let mut resolved = true;
    for p in obj.children() {
        if p.kind() == j::SPREAD_ELEMENT {
            resolved &= p
                .children()
                .next()
                .is_some_and(|a| collect_defaults(cx, &a, names, seen));
        } else if let Some(name) = js::member(&p).and_then(|m| static_name(&m)) {
            names.push(name);
        } else {
            resolved = false;
        }
    }
    resolved
}

fn macro_props(cx: &Cx, m: &DefineProps) -> Vec<Prop> {
    let d = defaults(cx, m);
    let mut out = Vec::new();
    if let Some(a) = args(&m.call).next() {
        if let Some(o) = object_or_array(cx, &a, 0) {
            runtime_props(&o, &d, &mut out);
        }
    } else if let Some(t) = js::type_arg(&m.call) {
        let mut bodies = Vec::new();
        flatten(cx, &t, &mut bodies, &mut Vec::new());
        for body in bodies {
            type_props(cx, &body, &d, &mut out);
        }
    }
    out
}

const NATIVE: &[&str] = &[
    "String", "Number", "Boolean", "Function", "Object", "Array", "Symbol",
];

fn is_boolean(n: &SyntaxNode) -> bool {
    match n.kind() {
        j::IDENTIFIER_REFERENCE => ident(n).is_some_and(|i| i == "Boolean"),
        j::ARRAY_EXPRESSION => {
            let mut items = n.children().filter(|c| c.kind() != j::ELISION);
            items
                .next()
                .is_some_and(|i| is_boolean(&i) && i.kind() == j::IDENTIFIER_REFERENCE)
                && items.next().is_none()
        }
        _ => false,
    }
}

fn has_default(obj: &SyntaxNode, name: &str) -> bool {
    js::property(obj, name).is_some()
}

fn without_default(value: &SyntaxNode) -> bool {
    match value.kind() {
        j::OBJECT_EXPRESSION => {
            let required = js::property(value, "required").is_some_and(|(_, m)| {
                m.value.is_some_and(|v| {
                    v.kind() == j::BOOLEAN_LITERAL && js::first(&v).as_deref() == Some("true")
                })
            });
            !required && !has_default(value, "default")
        }
        j::IDENTIFIER_REFERENCE => ident(value).is_some_and(|n| NATIVE.contains(&n.as_str())),
        j::CALL_EXPRESSION | j::STATIC_MEMBER_EXPRESSION | j::COMPUTED_MEMBER_EXPRESSION => false,
        _ => true,
    }
}

fn is_boolean_prop(value: &SyntaxNode) -> bool {
    is_boolean(value)
        || value.kind() == j::OBJECT_EXPRESSION
            && members(value).any(|(_, m)| {
                m.key.kind() == j::IDENTIFIER_NAME
                    && js::first(&m.key).as_deref() == Some("type")
                    && m.value.is_some_and(|v| is_boolean(&v))
            })
}

/// `type` and `validator` of the object form: is the type declared?
fn has_type(value: &SyntaxNode) -> Option<bool> {
    match value.kind() {
        j::OBJECT_EXPRESSION => Some(
            js::property(value, "type").is_some_and(|(_, m)| {
                m.value.is_none_or(|v| {
                    v.kind() != j::ARRAY_EXPRESSION || v.children().next().is_some()
                })
            }) || js::property(value, "validator").is_some(),
        ),
        j::ARRAY_EXPRESSION => Some(value.children().next().is_some()),
        j::FUNCTION | j::ARROW_FUNCTION_EXPRESSION => Some(false),
        _ => None,
    }
}

/// The props of an object or array (`defineProps({})`, the `props` option).
fn runtime_props(def: &SyntaxNode, d: &Defaults, out: &mut Vec<Prop>) {
    if def.kind() == j::ARRAY_EXPRESSION {
        for el in def.children().filter(|e| e.kind() != j::ELISION) {
            let name = string(&el);
            out.push(Prop {
                label: name
                    .clone()
                    .or_else(|| ident(&el))
                    .unwrap_or_else(|| "Unknown prop".into()),
                name,
                node: el,
                no_default: false,
                no_type: true,
            });
        }
        return;
    }
    for (node, m) in members(def) {
        let name = static_name(&m);
        let value = m.value.clone().map(js::unwrap);
        let no_default = !m.shorthand
            && value
                .as_ref()
                .is_some_and(|v| without_default(v) && !is_boolean_prop(v))
            && !d.ignores(name.as_deref(), false);
        let no_type = value.as_ref().is_some_and(|v| has_type(v) == Some(false));
        out.push(Prop {
            label: name
                .clone()
                .unwrap_or_else(|| format!("[{}]", node_text(&m.key))),
            name,
            node,
            no_default,
            no_type,
        });
    }
}

fn node_text(n: &SyntaxNode) -> String {
    n.text().to_string().trim().to_string()
}

/// The interface bodies and type literals a type is made of: unions and intersections taken
/// apart, names of interfaces and aliases of the file followed. Anything else is unknown.
fn flatten(cx: &Cx, t: &SyntaxNode, out: &mut Vec<SyntaxNode>, seen: &mut Vec<SyntaxNode>) {
    match t.kind() {
        j::TS_UNION_TYPE | j::TS_INTERSECTION_TYPE | j::TS_PARENTHESIZED_TYPE => {
            for c in t.children() {
                flatten(cx, &c, out, seen);
            }
        }
        j::TS_TYPE_LITERAL => out.push(t.clone()),
        j::TS_TYPE_REFERENCE => {
            let Some(decl) = t
                .children()
                .next()
                .and_then(|n| ident(&n))
                .and_then(|n| cx.only_decl(&n, true))
            else {
                return;
            };
            match decl.kind() {
                j::TS_INTERFACE_DECLARATION => out.extend(decl.child(j::TS_INTERFACE_BODY)),
                j::TS_TYPE_ALIAS_DECLARATION if !seen.contains(&decl) => {
                    seen.push(decl.clone());
                    if let Some(ty) = decl.children().last() {
                        flatten(cx, &ty, out, seen);
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

const STRING: u8 = 1;
const NUMBER: u8 = 2;
const BOOLEAN: u8 = 4;
const FUNCTION: u8 = 8;
const ARRAY: u8 = 16;
const OBJECT: u8 = 32;
const SYMBOL: u8 = 64;
/// A type that cannot be told (`undefined`, `any`, a name that is not known).
const UNKNOWN: u8 = 128;

/// The runtime types a type annotation stands for, as a set of the constants above.
fn infer(cx: &Cx, t: &SyntaxNode, seen: &mut Vec<SyntaxNode>) -> u8 {
    match t.kind() {
        j::TS_STRING_KEYWORD | j::TS_TEMPLATE_LITERAL_TYPE => STRING,
        j::TS_NUMBER_KEYWORD => NUMBER,
        j::TS_BOOLEAN_KEYWORD => BOOLEAN,
        j::TS_OBJECT_KEYWORD => OBJECT,
        j::TS_FUNCTION_TYPE => FUNCTION,
        j::TS_ARRAY_TYPE | j::TS_TUPLE_TYPE => ARRAY,
        j::TS_SYMBOL_KEYWORD => SYMBOL,
        j::TS_TYPE_LITERAL => {
            t.children().fold(0, |bits, m| {
                bits | if matches!(
                    m.kind(),
                    j::TS_CALL_SIGNATURE_DECLARATION | j::TS_CONSTRUCT_SIGNATURE_DECLARATION
                ) {
                    FUNCTION
                } else {
                    OBJECT
                }
            }) | if t.children().next().is_none() {
                OBJECT
            } else {
                0
            }
        }
        j::TS_LITERAL_TYPE => match t.children().next().map(|l| l.kind()) {
            Some(j::BOOLEAN_LITERAL) => BOOLEAN,
            Some(j::STRING_LITERAL) => STRING,
            Some(j::NUMERIC_LITERAL | j::BIG_INT_LITERAL) => NUMBER,
            _ => UNKNOWN,
        },
        j::TS_PARENTHESIZED_TYPE => t.children().next().map_or(UNKNOWN, |c| infer(cx, &c, seen)),
        j::TS_UNION_TYPE | j::TS_INTERSECTION_TYPE => {
            t.children().fold(0, |bits, c| bits | infer(cx, &c, seen))
        }
        j::TS_TYPE_REFERENCE => reference(cx, t, seen),
        _ => UNKNOWN,
    }
}

fn reference(cx: &Cx, t: &SyntaxNode, seen: &mut Vec<SyntaxNode>) -> u8 {
    let Some(name) = t.children().next().and_then(|n| ident(&n)) else {
        return UNKNOWN;
    };
    if let Some(decl) = cx.only_decl(&name, true) {
        match decl.kind() {
            j::TS_INTERFACE_DECLARATION => return OBJECT,
            j::TS_ENUM_DECLARATION => return enum_type(&decl),
            j::TS_TYPE_ALIAS_DECLARATION if !seen.contains(&decl) => {
                seen.push(decl.clone());
                return decl
                    .children()
                    .last()
                    .map_or(UNKNOWN, |ty| infer(cx, &ty, seen));
            }
            _ => {}
        }
    }
    let params: Vec<SyntaxNode> = t
        .child(j::TS_TYPE_PARAMETER_INSTANTIATION)
        .map(|i| i.children().collect())
        .unwrap_or_default();
    let param = |i: usize, seen: &mut Vec<SyntaxNode>| {
        params.get(i).map_or(UNKNOWN, |p| infer(cx, p, seen))
    };
    match name.strip_prefix("Readonly").unwrap_or(&name) {
        "Array" | "Function" | "Object" | "Set" | "Map" | "WeakSet" | "WeakMap" | "Date" => {
            return OBJECT;
        }
        _ => {}
    }
    match name.as_str() {
        "Record" | "Partial" | "Readonly" | "Pick" | "Omit" | "Required" | "InstanceType" => OBJECT,
        "Uppercase" | "Lowercase" | "Capitalize" | "Uncapitalize" => STRING,
        "Parameters" | "ConstructorParameters" => ARRAY,
        "NonNullable" => param(0, seen) & !UNKNOWN,
        "Extract" => param(1, seen),
        "Exclude" | "OmitThisParameter" => param(0, seen),
        _ => UNKNOWN,
    }
}

fn enum_type(decl: &SyntaxNode) -> u8 {
    let bits = decl
        .child(j::TS_ENUM_BODY)
        .into_iter()
        .flat_map(|b| b.children())
        .filter_map(|m| m.children().nth(1))
        .fold(0, |bits, init| {
            bits | match init.kind() {
                j::STRING_LITERAL => STRING,
                j::NUMERIC_LITERAL | j::BIG_INT_LITERAL => NUMBER,
                _ => UNKNOWN,
            }
        });
    if bits == 0 { NUMBER } else { bits }
}

/// The props of an interface body or a type literal.
fn type_props(cx: &Cx, body: &SyntaxNode, d: &Defaults, out: &mut Vec<Prop>) {
    for m in body.children() {
        let method = match m.kind() {
            j::TS_PROPERTY_SIGNATURE => false,
            j::TS_METHOD_SIGNATURE => true,
            _ => continue,
        };
        let Some(key) = m
            .children()
            .next()
            .filter(|k| {
                matches!(
                    k.kind(),
                    j::IDENTIFIER_NAME | j::STRING_LITERAL | j::NUMERIC_LITERAL
                )
            })
            .filter(|_| !m.tokens().any(|t| t.kind() == j::L_BRACKET))
        else {
            continue;
        };
        let name = key_name(&key);
        let required = !m.tokens().any(|t| t.kind() == j::QUESTION);
        let bits = if method {
            FUNCTION
        } else {
            m.child(j::TS_TYPE_ANNOTATION)
                .and_then(|a| a.children().next())
                .map_or(UNKNOWN, |t| infer(cx, &t, &mut Vec::new()))
        };
        out.push(Prop {
            no_default: !(required || bits == BOOLEAN) && !d.ignores(name.as_deref(), true),
            label: name.clone().unwrap_or_default(),
            name,
            node: m,
            no_type: false,
        });
    }
}
