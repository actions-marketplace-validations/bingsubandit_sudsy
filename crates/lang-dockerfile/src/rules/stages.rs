//! Rules about the build stages of a file: what each FROM starts and what refers to it.
//!
//! concept: new untagged-base-image, new last-user-not-root, new copy-from-unknown-stage, new copy-from-own-stage, new from-first, new build-stage-checks
//! what: untagged-base-image: A FROM image with neither a tag nor a digest, which builds on whatever `latest` is
//! what: last-user-not-root: A stage whose last USER is root (`root`, `0`, `root:group`, `0:group`)
//! what: copy-from-unknown-stage: A `COPY --from` that names no earlier stage: not a stage name, not the index of an earlier stage, and not an image (no `:`)
//! what: copy-from-own-stage: A `COPY --from` that names the stage it is in
//! what: from-first: An instruction other than ARG before the first FROM
//! what: build-stage-checks: What each FROM starts and what refers to it.
//! tools: build-stage-checks: hadolint DL3006

use crate::generated::kind as k;
use core_rules::tree::significant_range;
use core_rules::unknown_reference::{self, Item, Scope};
use core_rules::{Family, Params, Rule, RuleContext, RuleMeta, rule};
use core_syntax::{SyntaxKind, SyntaxNode, SyntaxNodeExt, TextRange};

/// A top-level instruction: its upper-case name; the arguments are read when a rule asks.
pub(super) struct Inst {
    pub name: String,
    pub node: SyntaxNode,
}

fn is_blank(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        k::WHITESPACE | k::NEWLINE | k::COMMENT | k::LINE_CONTINUATION
    )
}

pub(super) fn instructions(doc: &SyntaxNode) -> Vec<Inst> {
    doc.children()
        .filter(|n| n.kind() == k::INSTRUCTION)
        .map(|node| {
            let name = node
                .tokens()
                .find(|t| !is_blank(t.kind()))
                .map_or(String::new(), |t| t.text().to_ascii_uppercase());
            Inst { name, node }
        })
        .collect()
}

impl Inst {
    pub fn range(&self, ctx: &RuleContext<'_>) -> TextRange {
        significant_range(&self.node, ctx.file.grammar)
    }

    /// The arguments as written (no comments or line continuations); tokens with no blank between
    /// them are one word (`--platform=` and `$ARCH`).
    pub fn words(&self) -> Vec<String> {
        let mut words: Vec<String> = Vec::new();
        let mut end = None;
        for t in self.node.tokens() {
            if is_blank(t.kind()) {
                end = None;
                continue;
            }
            match (end, words.last_mut()) {
                (Some(e), Some(w)) if e == t.text_range().start() => w.push_str(t.text()),
                _ => words.push(t.text().to_string()),
            }
            end = Some(t.text_range().end());
        }
        words.remove(0);
        words
    }

    /// `--name=value` of the instruction.
    fn option(&self, name: &str) -> Option<String> {
        self.words().into_iter().find_map(|w| {
            Some(
                w.strip_prefix("--")?
                    .strip_prefix(name)?
                    .strip_prefix('=')?
                    .to_string(),
            )
        })
    }

    /// The image and the stage name of a FROM.
    fn from(&self) -> (String, Option<String>) {
        let words = self.words();
        let mut rest = words.into_iter().filter(|w| !w.starts_with("--"));
        let image = rest.next().unwrap_or_default();
        let alias = match (rest.next(), rest.next()) {
            (Some(a), Some(n)) if a.eq_ignore_ascii_case("as") => Some(n),
            _ => None,
        };
        (image, alias)
    }
}

rule! {
    pub UntaggedBaseImage {
        id: "untagged-base-image",
        category: Suspicious,
        default: Off,
        description: "Reports a `FROM` image with neither a tag nor a digest (`FROM alpine`). Docker reads it as \
                  `:latest`. Not reported: `scratch`, a variable (`$BASE`) and the name of an earlier stage \
                  (compared without case). A `:` in the registry host (`localhost:5000/app`) is a port, not a \
                  tag.",
        why: "An image without a tag is pulled as `latest`, so the Dockerfile does not even say which \
                  version it builds on. The base changes with each upstream release, and a rebuild can fail or \
                  behave differently with no change in the file.\n\
                  Name a release (`alpine:3.20`) or a digest.",
        example: "FROM alpine   # FROM alpine:3.20",
        kinds: &[k::DOCUMENT],
    }

    fn check(&self, doc: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let mut stages: Vec<String> = Vec::new();
        for i in instructions(doc).iter().filter(|i| i.name == "FROM") {
            let (image, alias) = i.from();
            // A `:` before the last `/` is a registry port, not a tag.
            let tagged = image.rsplit('/').next().is_some_and(|name| name.contains(':')) || image.contains('@');
            if !tagged && image != "scratch" && !image.starts_with('$') && !stages.contains(&image.to_lowercase()) {
                ctx.report(i.range(ctx), format!("`{image}` has no tag"));
            }
            stages.extend(alias.map(|a| a.to_lowercase()));
        }
    }
}

rule! {
    pub LastUserNotRoot {
        id: "last-user-not-root",
        category: Correctness,
        default: Warn,
        description: "Reports a stage whose last `USER` is root, on that `USER`. That is `root` or `0`, also \
                  `root:group` and `0:group`. Every stage is checked, build stages too. A stage with no `USER` \
                  instruction is not reported.",
        why: "The container's main process runs as the last user. If the application is compromised, the \
                  attacker is root inside the container, which makes it easier to damage mounted volumes or to \
                  break out of the container, and files the process writes to a volume are owned by root.\n\
                  End the stage with an unprivileged `USER` (a numeric one, see `numeric-user-id`).",
        example: "USER root   # last USER of the stage",
        kinds: &[k::DOCUMENT],
    }

    fn check(&self, doc: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let mut root = None;
        let flush = |root: &mut Option<TextRange>, ctx: &mut RuleContext<'_>| {
            if let Some(r) = root.take() {
                ctx.report(r, "the last USER of the stage is root");
            }
        };
        let mut seen_from = false;
        for i in instructions(doc) {
            match i.name.as_str() {
                "FROM" => {
                    flush(&mut root, ctx);
                    seen_from = true;
                }
                "USER" if seen_from => {
                    let words = i.words();
                    let user = words.first().map_or("", String::as_str);
                    let name = user.split(':').next().unwrap_or("");
                    root = (name == "root" || name == "0").then_some(i.range(ctx));
                }
                _ => {}
            }
        }
        flush(&mut root, ctx);
    }
}

rule! {
    pub CopyFromUnknownStage {
        id: "copy-from-unknown-stage",
        category: Correctness,
        default: Warn,
        description: "Reports a `COPY --from=name` that names no stage defined earlier. Not reported: the number \
                  of an earlier stage (`--from=0`) and an image reference with a `:` (`--from=nginx:1`). A \
                  stage defined further down does not count. Names are compared without regard to case.",
        why: "Docker does not fail on an unknown name: it takes it for an image and pulls it from the \
                  registry. A typo in a stage name (`buidler`) or a reference to a stage defined further down \
                  copies files from whatever image has that name, or fails only when the pull fails.",
        example: "COPY --from=build /a /b   # no stage `build` before this line",
        kinds: &[k::DOCUMENT],
    }

    fn check(&self, doc: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let (all, grammar) = (instructions(doc), ctx.file.grammar);
        let mut items = Vec::with_capacity(all.len());
        for (tag, i) in all.iter().enumerate() {
            if i.name == "FROM" {
                items.push(Item::Declare(i.from().1.unwrap_or_default()));
            } else if i.name == "COPY" {
                items.extend(i.option("from").map(|name| Item::Use { name, tag }));
            }
        }
        // An image (`name:tag`) and the index of an earlier stage need no stage name.
        let exempt = |from: &str, count: usize| from.contains(':') || from.parse::<usize>().is_ok_and(|n| n < count);
        unknown_reference::check(&items, &Scope { earlier_only: true, ignore_case: true }, exempt, |tag, from| (significant_range(&all[tag].node, grammar), format!("`COPY --from` names no earlier stage: `{from}`")), ctx);
    }
}

rule! {
    pub CopyFromOwnStage {
        id: "copy-from-own-stage",
        category: Correctness,
        default: Error,
        description: "Reports a `COPY --from` that names its own stage. That is `COPY --from=build` in the stage \
                  that `FROM alpine AS build` starts. Names are compared without regard to case.",
        why: "A stage cannot copy from itself: its content does not exist yet while it is built, and \
                  BuildKit stops with a circular dependency error. The source is usually an earlier stage, or \
                  the build context (then drop `--from`).",
        example: "FROM a AS x\nCOPY --from=x /a /b   # reported",
        kinds: &[k::DOCUMENT],
    }

    fn check(&self, doc: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let mut own: Option<String> = None;
        for i in instructions(doc) {
            if i.name == "FROM" {
                own = i.from().1;
            } else if i.name == "COPY" && i.option("from").is_some_and(|f| own.as_ref().is_some_and(|o| o.eq_ignore_ascii_case(&f))) {
                ctx.report(i.range(ctx), "a stage cannot copy from itself");
            }
        }
    }
}

rule! {
    pub FromFirst {
        id: "from-first",
        category: Correctness,
        default: Error,
        description: "Reports an instruction other than `ARG` before the first `FROM`. Comments, parser directives \
                  and blank lines are allowed there.",
        why: "Every instruction works on a build stage, and a stage starts with `FROM`. Only `ARG` may \
                  come first, because its value can be used in the `FROM` line (`ARG VERSION=3` then `FROM \
                  alpine:${VERSION}`). Any other instruction first makes the Dockerfile invalid, and Docker \
                  rejects it.",
        example: "RUN echo hi\nFROM alpine:3   # the RUN comes first",
        kinds: &[k::DOCUMENT],
    }

    fn check(&self, doc: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        for i in instructions(doc) {
            match i.name.as_str() {
                "FROM" => return,
                "ARG" => {}
                _ => {
                    ctx.report(i.range(ctx), "the file must begin with FROM or ARG");
                }
            }
        }
    }
}

pub const ONCE_PER_STAGE: Family = Family {
    name: "once-per-stage",
    build,
};

/// `instruction`: the instruction that may appear once per stage (CMD, ENTRYPOINT, HEALTHCHECK);
/// each further one is reported.
struct OncePerStage {
    meta: &'static RuleMeta,
    message: String,
    instruction: String,
}

fn build(meta: &'static RuleMeta, mut p: Params) -> Result<Box<dyn Rule>, String> {
    let rule = OncePerStage {
        meta,
        message: p.text("message")?,
        instruction: p.text("instruction")?.to_ascii_uppercase(),
    };
    p.done()?;
    Ok(Box::new(rule))
}

impl Rule for OncePerStage {
    fn meta(&self) -> &'static RuleMeta {
        self.meta
    }

    fn kinds(&self) -> &[SyntaxKind] {
        &[k::DOCUMENT]
    }

    fn check(&self, doc: &SyntaxNode, ctx: &mut RuleContext<'_>) {
        let mut seen = false;
        for i in instructions(doc) {
            if i.name == "FROM" {
                seen = false;
            } else if i.name == self.instruction {
                if seen {
                    ctx.report(i.range(ctx), self.message.clone());
                }
                seen = true;
            }
        }
    }
}
