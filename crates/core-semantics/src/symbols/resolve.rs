//! Name resolution in a file: a written name becomes the qualified names it can stand for, from
//! the namespace, the imports and aliases, the enclosing scopes and the language's options
//! (separator, absolute marker, case folding and global fallback per name kind, lookup through
//! every enclosing scope or the namespace only).
//!
//! concept: name-resolution
//! what: A written name to the qualified names it can stand for, through namespace, imports and enclosing scopes.
//! tools: PHPStan name resolution, protoc symbol lookup
//! reads: file:symbols.rows

use super::rows::{Facts, NameKind};
use std::borrow::Cow;

/// An import: `use A\B as C;` is the alias `c` (folded as the language folds) of the target
/// `A\B`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alias {
    pub kind: NameKind,
    pub name: String,
    pub target: String,
}

impl Facts {
    /// Whether the declaring call `name` (as the file wrote it, lower case) may declare a name of
    /// `kind`.
    pub fn call_declares(&self, name: &str, kind: NameKind) -> bool {
        self.declaring_calls
            .iter()
            .any(|c| c.name == name && c.kind.is_none_or(|k| k == kind))
    }

    fn folds(&self, kind: NameKind) -> bool {
        self.fold & kind.bit() != 0
    }

    /// The key a name is compared by: lower case where the language folds that kind.
    pub fn key<'a>(&self, kind: NameKind, name: &'a str) -> Cow<'a, str> {
        if self.folds(kind) && name.bytes().any(|b| b.is_ascii_uppercase()) {
            Cow::Owned(name.to_ascii_lowercase())
        } else {
            Cow::Borrowed(name)
        }
    }

    /// `scope` and `name` joined by the separator.
    pub fn qualify(&self, scope: &str, name: &str) -> String {
        if scope.is_empty() {
            name.to_string()
        } else {
            format!("{scope}{}{name}", self.separator)
        }
    }

    /// The qualified names `written` can stand for at a place whose scope is `scope` (the
    /// namespace, or the innermost enclosing type or package in a language that looks through
    /// every enclosing scope), given the imports `aliases` in force. The first that is declared
    /// is the one meant.
    pub fn resolve(
        &self,
        written: &str,
        kind: NameKind,
        scope: &str,
        aliases: &[Alias],
    ) -> Vec<String> {
        let sep = self.separator.as_str();
        if !self.absolute.is_empty()
            && let Some(rest) = written.strip_prefix(self.absolute.as_str())
        {
            return vec![rest.to_string()];
        }
        let (first, rest) = match written.split_once(sep) {
            Some((first, rest)) => (first, Some(rest)),
            None => (written, None),
        };
        if !self.relative.is_empty() && first == self.relative && rest.is_some() {
            return vec![self.qualify(scope, rest.unwrap_or_default())];
        }
        let alias = |kind: NameKind, name: &str| {
            let key = self.key(kind, name);
            aliases
                .iter()
                .find(|a| a.kind == kind && a.name == key)
                .map(|a| a.target.as_str())
        };
        let mut found: Vec<String> = Vec::new();
        match rest {
            // A qualified name: only its first segment is looked up, among the imported types.
            Some(rest) => {
                if let Some(target) = alias(NameKind::Type, first) {
                    found.push(format!("{target}{sep}{rest}"));
                }
            }
            None => {
                if let Some(target) = alias(kind, first) {
                    found.push(target.to_string());
                }
            }
        }
        if !found.is_empty() {
            return found;
        }
        if self.lexical {
            let mut scope = scope;
            loop {
                found.push(self.qualify(scope, written));
                match scope.rfind(sep) {
                    Some(at) => scope = &scope[..at],
                    None if scope.is_empty() => break,
                    None => scope = "",
                }
            }
        } else {
            found.push(self.qualify(scope, written));
            if rest.is_none() && self.fallback & kind.bit() != 0 && !scope.is_empty() {
                found.push(written.to_string());
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn php() -> Facts {
        Facts {
            separator: "\\".into(),
            absolute: "\\".into(),
            relative: "namespace".into(),
            fold: NameKind::Type.bit() | NameKind::Function.bit(),
            fallback: NameKind::Function.bit() | NameKind::Constant.bit(),
            ..Facts::default()
        }
    }

    fn alias(kind: NameKind, name: &str, target: &str) -> Alias {
        Alias {
            kind,
            name: name.into(),
            target: target.into(),
        }
    }

    #[test]
    fn a_namespaced_language_looks_in_the_namespace_then_the_imports() {
        let f = php();
        let uses = [
            alias(NameKind::Type, "baz", "App\\Contracts\\Bar"),
            alias(NameKind::Function, "h", "App\\helper"),
        ];
        let r = |w, k| f.resolve(w, k, "App\\Models", &uses);
        assert_eq!(r("\\Foo", NameKind::Type), ["Foo"]);
        assert_eq!(r("Baz", NameKind::Type), ["App\\Contracts\\Bar"]);
        assert_eq!(r("BAZ\\X", NameKind::Type), ["App\\Contracts\\Bar\\X"]);
        assert_eq!(r("Other", NameKind::Type), ["App\\Models\\Other"]);
        assert_eq!(r("h", NameKind::Function), ["App\\helper"]);
        assert_eq!(
            r("strlen", NameKind::Function),
            ["App\\Models\\strlen", "strlen"]
        );
        assert_eq!(r("namespace\\X", NameKind::Type), ["App\\Models\\X"]);
        // A qualified function name does not fall back.
        assert_eq!(r("a\\f", NameKind::Function), ["App\\Models\\a\\f"]);
    }

    #[test]
    fn a_language_with_nested_scopes_tries_every_enclosing_scope() {
        let f = Facts {
            separator: ".".into(),
            absolute: ".".into(),
            lexical: true,
            ..Facts::default()
        };
        let r = |w| f.resolve(w, NameKind::Type, "foo.bar.M", &[]);
        assert_eq!(r("N"), ["foo.bar.M.N", "foo.bar.N", "foo.N", "N"]);
        assert_eq!(r(".x.Y"), ["x.Y"]);
        assert_eq!(f.resolve("N", NameKind::Type, "", &[]), ["N"]);
    }

    #[test]
    fn keys_fold_the_kinds_a_language_folds() {
        let f = php();
        assert_eq!(f.key(NameKind::Type, "App\\Foo"), "app\\foo");
        assert_eq!(f.key(NameKind::Constant, "App\\Foo"), "App\\Foo");
    }
}
