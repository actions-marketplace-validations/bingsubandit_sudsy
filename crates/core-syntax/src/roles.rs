//! The role vocabulary: what a node kind is, for code that works across languages (layout kits,
//! scope binders, shared rules). A grammar gives a node kind a role by tagging it `#<role>`, or
//! marks it `#opaque` when no role fits; a grammar that declares `roles` must do one of the two
//! for every node kind (`Grammar::check_roles`, run by `cargo xtask codegen`). A kind may carry
//! several roles (a method is a declaration and a function).
//!
//! `fields` lists the named parts shared code reads through `Grammar::slot`; a name ending in `?`
//! is optional. A required field must be declared in the node's body, as the item's label or as
//! an alias (`when_true=then:#expression`); the item itself may be optional or repeated.

pub struct Role {
    pub name: &'static str,
    pub fields: &'static str,
    pub doc: &'static str,
}

pub const OPAQUE: &str = "opaque";

pub const ROLES: &[Role] = &[
    Role {
        name: "call",
        fields: "callee arguments",
        doc: "calls a function or method",
    },
    Role {
        name: "member-access",
        fields: "member object?",
        doc: "`object.member`; the object is absent in a binding (`.member`)",
    },
    Role {
        name: "index",
        fields: "arguments object?",
        doc: "`object[arguments]`; the object is absent in a binding",
    },
    Role {
        name: "binary",
        fields: "left operator right",
        doc: "two operands and an operator, any precedence",
    },
    Role {
        name: "unary",
        fields: "operand operator?",
        doc: "prefix, postfix or keyword operator on one operand",
    },
    Role {
        name: "assignment",
        fields: "left operator right",
        doc: "stores a value, plain or compound",
    },
    Role {
        name: "conditional",
        fields: "condition then else",
        doc: "ternary expression",
    },
    Role {
        name: "function",
        fields: "parameters body? expression_body?",
        doc: "method, lambda or local function",
    },
    Role {
        name: "parameter-list",
        fields: "parameters",
        doc: "the parameters of a function with their brackets",
    },
    Role {
        name: "block",
        fields: "items",
        doc: "braced sequence of statements or members",
    },
    Role {
        name: "declaration",
        fields: "name?",
        doc: "introduces a name (member, accessor, namespace)",
    },
    Role {
        name: "type-declaration",
        fields: "members name?",
        doc: "class, struct, interface, enum",
    },
    Role {
        name: "import",
        fields: "path",
        doc: "brings the names of another file or namespace into scope",
    },
    Role {
        name: "literal",
        fields: "value",
        doc: "a constant written in the source",
    },
    Role {
        name: "scope",
        fields: "",
        doc: "names declared inside are not visible outside",
    },
    Role {
        name: "reference",
        fields: "name",
        doc: "uses a name",
    },
];

pub fn role(name: &str) -> Option<&'static Role> {
    ROLES.iter().find(|r| r.name == name)
}

impl Role {
    /// The fields as (name, required).
    pub fn fields(&self) -> impl Iterator<Item = (&'static str, bool)> {
        self.fields
            .split_whitespace()
            .map(|f| f.strip_suffix('?').map_or((f, true), |n| (n, false)))
    }
}
