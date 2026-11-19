# Review: does the diff re-implement something in the index?
The lead runs this on every language step with a fresh reviewer that sees only `docs/ai/kit/index.md` and the diff (`git diff origin/main`), not the author's reasoning.

Prompt:
> You get the kit index (one row per concept: id, module, what, tools, used by, flags) and a diff. For every added or changed hunk of Rust in a `lang-*` crate, say which index entries it re-implements or could have used instead, with the row id and one line why. Also list added `new` concepts whose `what` matches an existing row. Answer "none" per hunk when nothing matches. Do not review style or correctness.

Lead's checks on the answer: each match becomes a fix in the step (use the shared form, or adopt: move both languages onto it) or a reason that it is a fact of the language; a step whose matches are not fixed or justified is rejected. Compare with `cargo xtask codegen --check` (headers, LOCAL entries) and `cargo xtask dupes --check`.
