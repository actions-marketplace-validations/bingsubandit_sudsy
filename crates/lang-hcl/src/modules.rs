//! The three Terraform module rules (tflint's terraform_unused_declarations,
//! terraform_required_providers, terraform_required_version): the words of Terraform for the
//! shared directory checks.
//!
//! concept: module-names

use core_rules::module_names::{Declare, Names, Provider, Setting};
use core_rules::{DirectoryScope, names_rule};

static TERRAFORM: Names = Names {
    scope: DirectoryScope {
        members: &[".tf"],
        opaque: &[".tf.json"],
        merged: &["override.tf"],
    },
    block: "Block",
    body: "Body",
    head: "type",
    labels: "labels",
    reference: "VariableExpr",
    reference_member: "GetAttr",
    declare: Declare {
        labels: &["variable", "data"],
        members: &["locals"],
        alias: &["provider"],
        alias_member: "alias",
        rename: &[("variable", "var"), ("locals", "local")],
        two_parts: &["data"],
    },
    provider: Provider {
        use_heads: &["resource", "data"],
        name_heads: &["provider"],
        use_member: "provider",
        separator: "_",
        provide_head: "terraform",
        provide_block: "required_providers",
        exempt: &["terraform"],
        entry_key: "version",
    },
    setting: Setting {
        head: "terraform",
        name: "required_version",
        anchor: "main.tf",
    },
};

names_rule! {
    pub UNUSED_DECLARATIONS {
        id: "unused-declarations",
        category: Suspicious,
        default: Off,
        description: "Reports a variable, local value, data source or provider alias that no file of the \
                  module (the `.tf` files of the directory) refers to. Override files declare nothing \
                  of their own. A module with `.tf.json` files, a file that does not parse or one that \
                  cannot be read is not judged.",
        why: "An unused declaration is left over from a change. A variable that nothing reads \
                  misleads the user of the module, who sets it and sees no effect, and an unused data \
                  source still makes a request on each plan.\n\
                  To fix, delete it.",
        example: "variable \"unused\" {}",
        check: Unused,
        message: "`{name}` is declared but not used",
        names: &TERRAFORM,
    }
}

names_rule! {
    pub REQUIRED_PROVIDERS {
        id: "required-providers",
        category: Style,
        default: Off,
        description: "Reports a provider that a `resource`, `data` or `provider` block uses and that no \
                  `required_providers` entry of the module names, on the last such block of the \
                  provider, and a `required_providers` entry without a `version`. The built-in \
                  `terraform` provider is exempt, and override files do not count.",
        why: "Without the declaration Terraform picks the source of the provider by the default \
                  for its name, and without a version it takes the newest, so two runs of the same \
                  code can use different providers and a new major version can break the module with \
                  no change in the code.\n\
                  To fix, add the provider to `required_providers` with a `source` and a version \
                  constraint.",
        example: "resource \"aws_instance\" \"a\" {}",
        check: MissingProvider,
        message: "Missing version constraint for provider `{name}` in `required_providers`",
        names: &TERRAFORM,
    }
}

names_rule! {
    pub REQUIRED_VERSION {
        id: "required-version",
        category: Style,
        default: Off,
        description: "Reports a module none of whose `terraform` blocks sets `required_version`, on the \
                  `terraform` block, or on the start of `main.tf` when the module has no such block. \
                  Override files do not count.",
        why: "Terraform language features and the state format change between versions, and a \
                  state file written by a newer version cannot be used by an older one. A constraint \
                  makes an incompatible version fail at the start with a clear message instead of in \
                  the middle of an apply.\n\
                  To fix, set `required_version` in a `terraform` block.",
        example: "terraform { backend \"s3\" {} }",
        check: MissingSetting,
        message: "the module's `terraform` blocks have no `required_version`",
        names: &TERRAFORM,
    }
}
