//! Rules about how an expression is written, each with a safe fix: `nullable-shorthand`,
//! `use-lambda` and `use-literal-suffix`.
//!
//! concept: new nullable-shorthand, new use-lambda, new use-literal-suffix
//! what: nullable-shorthand: `int?` instead of `Nullable<int>` or `System.Nullable<int>`
//! what: use-lambda: A lambda instead of an anonymous method
//! what: use-literal-suffix: A numeric literal cast to `long`, `ulong`, `uint`, `float`, `double` or `decimal` when the literal's own type differs: `(long)1` is `1L`

use super::{has_comment_in, kind, text_of};
use core_rules::rule;
use core_rules::tree::significant_range;
use core_rules::{Edit, RuleContext};
use core_syntax::{SyntaxNode, SyntaxNodeExt, TextRange};

/// Is `left` `System` or `global::System`?
fn is_system(left: &SyntaxNode, ctx: &RuleContext<'_>) -> bool {
    matches!(text_of(left, ctx), "System" | "global::System")
}

rule! {
    pub NullableShorthand {
        id: "nullable-shorthand",
        category: Style,
        default: Off,
        fix: Safe,
        description: "`int?` instead of `Nullable<int>` or `System.Nullable<int>`. Name-based: a \
                  type of your own called `Nullable` is reported too. Not reported in `using` \
                  aliases, `nameof`, `typeof(Nullable<>)` and before a member access \
                  (`Nullable<int>.Equals`).",
        example: "Nullable<int> x;   // write: int? x;",
        kinds: &[kind::GENERIC_NAME],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let is_nullable = node
            .tokens()
            .any(|t| t.kind() == kind::IDENTIFIER_TOKEN && t.text() == "Nullable");
        let Some(arguments) = node
            .child(kind::TYPE_ARGUMENT_LIST)
        else {
            return;
        };
        let mut types = arguments.children();
        let (Some(argument), None) = (types.next(), types.next()) else {
            return;
        };
        if !is_nullable || argument.kind() == kind::OMITTED_TYPE_ARGUMENT {
            return;
        }
        for ancestor in node.ancestors().skip(1) {
            let nameof = ancestor.kind() == kind::INVOCATION_EXPRESSION
                && ancestor
                    .children()
                    .next()
                    .is_some_and(|c| text_of(&c, ctx) == "nameof");
            if nameof || ancestor.kind() == kind::USING_DIRECTIVE {
                return;
            }
        }
        // The name with its `System.` qualifier, if it has one.
        let mut outer = node.clone();
        if let Some(parent) = node.parent()
            && matches!(
                parent.kind(),
                kind::QUALIFIED_NAME | kind::SIMPLE_MEMBER_ACCESS_EXPRESSION
            )
            && parent.children().last().as_ref() == Some(node)
        {
            if !parent.children().next().is_some_and(|l| is_system(&l, ctx)) {
                return;
            }
            outer = parent;
        }
        let range = significant_range(&outer, ctx.file.grammar);
        let argument_text = text_of(&argument, ctx);
        let before_member = outer.parent().is_some_and(|p| {
            matches!(
                p.kind(),
                kind::QUALIFIED_NAME | kind::SIMPLE_MEMBER_ACCESS_EXPRESSION
            ) && p.children().next().as_ref() == Some(&outer)
        });
        // `int?.Equals(..)` is not valid C#, and StyleCop leaves such static calls alone.
        if before_member {
            return;
        }
        ctx.report_fix(
            range,
            format!("use `{argument_text}?` instead of `Nullable<{argument_text}>`"),
            (
                "Use the nullable shorthand",
                vec![Edit::replace(range, format!("{argument_text}?"))],
            ),
        );
    }
}

rule! {
    pub UseLambda {
        id: "use-lambda",
        category: Style,
        default: Off,
        fix: Safe,
        description: "A lambda instead of an anonymous method. The fix is offered when the anonymous \
                  method has a parameter list and no comment before its body; `delegate { }` \
                  without one converts to delegate types with any parameters, so it has no fix.",
        example: "Func<int, int> f = delegate (int x) { return x; };   // write: (int x) => { return x; }",
        kinds: &[kind::ANONYMOUS_METHOD_EXPRESSION],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let Some(keyword) = node
            .token(kind::DELEGATE_KEYWORD)
        else {
            return;
        };
        let parameters = node.child(kind::PARAMETER_LIST);
        let fix = parameters.and_then(|p| {
            let end = significant_range(&p, ctx.file.grammar).end();
            let range = TextRange::new(keyword.text_range().start(), end);
            if has_comment_in(node, range, ctx) {
                return None;
            }
            let attached = ctx.file.text[usize::from(end)..]
                .chars()
                .next()
                .is_some_and(|c| !c.is_whitespace());
            let tail = if attached { " " } else { "" };
            let edit = Edit::replace(range, format!("{} =>{tail}", text_of(&p, ctx)));
            Some(("Use a lambda", vec![edit]))
        });
        ctx.report_fix(keyword.text_range(), "use lambda syntax", fix);
    }
}

/// The type of an integer or real literal and whether it is written in hexadecimal or binary,
/// from its text. `None` when it does not fit any type.
fn literal_type(text: &str) -> Option<(&'static str, bool)> {
    let lower = text.to_ascii_lowercase().replace('_', "");
    let radix = if lower.starts_with("0x") {
        Some(16)
    } else if lower.starts_with("0b") {
        Some(2)
    } else {
        None
    };
    let has_suffix = |s: &str| lower.ends_with(s);
    if radix.is_none() {
        if has_suffix("f") {
            return Some(("float", false));
        }
        if has_suffix("m") {
            return Some(("decimal", false));
        }
        if lower.contains('.') || lower.contains('e') || has_suffix("d") {
            return Some(("double", false));
        }
    }
    let digits = lower.trim_end_matches(['u', 'l']);
    let suffix = &lower[digits.len()..];
    let value = match radix {
        Some(base) => u128::from_str_radix(&digits[2..], base).ok()?,
        None => digits.parse::<u128>().ok()?,
    };
    let by_value = |signed_first: bool| {
        if signed_first && value <= i32::MAX as u128 {
            "int"
        } else if value <= u32::MAX as u128 {
            "uint"
        } else if signed_first && value <= i64::MAX as u128 {
            "long"
        } else {
            "ulong"
        }
    };
    let kind = match suffix {
        "" => by_value(true),
        "u" => by_value(false),
        "l" if value <= i64::MAX as u128 => "long",
        _ => "ulong",
    };
    Some((kind, radix.is_some()))
}

/// The suffix that gives a literal the type `target`.
fn suffix_of(target: &str) -> Option<&'static str> {
    Some(match target {
        "long" => "L",
        "ulong" => "UL",
        "uint" => "U",
        "float" => "F",
        "double" => "D",
        "decimal" => "M",
        _ => return None,
    })
}

rule! {
    pub UseLiteralSuffix {
        id: "use-literal-suffix",
        category: Style,
        default: Off,
        fix: Safe,
        description: "A numeric literal cast to `long`, `ulong`, `uint`, `float`, `double` or \
                  `decimal` when the literal's own type differs: `(long)1` is `1L`. Not reported \
                  for hexadecimal and binary literals cast to a floating-point type, which have \
                  no suffix.",
        example: "var x = (long)1;   // write: var x = 1L;",
        kinds: &[kind::CAST_EXPRESSION],
    }

    fn check(&self, node: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let mut parts = node.children();
        let (Some(target), Some(mut operand)) = (parts.next(), parts.last()) else {
            return;
        };
        let target = text_of(&target, ctx);
        let (Some(suffix), true) = (
            suffix_of(target),
            node.children()
                .next()
                .is_some_and(|t| t.kind() == kind::PREDEFINED_TYPE),
        ) else {
            return;
        };
        let mut sign = "";
        loop {
            match operand.kind() {
                kind::PARENTHESIZED_EXPRESSION => {}
                kind::UNARY_MINUS_EXPRESSION if sign.is_empty() => sign = "-",
                kind::UNARY_PLUS_EXPRESSION if sign.is_empty() => sign = "+",
                kind::NUMERIC_LITERAL_EXPRESSION => break,
                _ => return,
            }
            let Some(inner) = operand.children().next() else {
                return;
            };
            operand = inner;
        }
        let literal = text_of(&operand, ctx);
        let Some((literal_kind, radix)) = literal_type(literal) else {
            return;
        };
        let integer_target = matches!(target, "long" | "ulong" | "uint");
        let integer_literal = !matches!(literal_kind, "float" | "double" | "decimal");
        let skip = literal_kind == target
            || (integer_target && !integer_literal)
            || (radix && !integer_target)
            || (sign == "-" && matches!(target, "ulong" | "uint"));
        if skip {
            return;
        }
        let letters: &[char] = if radix {
            &['u', 'U', 'l', 'L']
        } else {
            &['u', 'U', 'l', 'L', 'f', 'F', 'd', 'D', 'm', 'M']
        };
        let bare = literal.trim_end_matches(letters);
        let range = significant_range(node, ctx.file.grammar);
        let fix = (!has_comment_in(node, range, ctx)).then(|| {
            let edit = Edit::replace(range, format!("{sign}{bare}{suffix}"));
            ("Use a literal suffix", vec![edit])
        });
        ctx.report_fix(
            range,
            format!("use the literal suffix `{suffix}` instead of casting to {target}"),
            fix,
        );
    }
}
