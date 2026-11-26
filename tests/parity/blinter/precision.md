# Precision of the Sudsy rules of the Blinter preset

Method: for each mapped rule, 20 findings of the dev run on the corpus (a seeded shuffle, `random.Random(11)`), each judged by hand.
A finding on a line where Blinter reports any code counts as true (`blinter` column; any code on the line, so a loose proxy: a line Blinter reports for another reason is counted as true); the others were read one by one (`judged`).
`no-trailing-whitespace`, `max-line-length` and `cmd-extension` are decided by a count, not sampled.
Rules below 90% are off by default (every rule here is off by default and on only in `preset:batch/blinter`).

| rule | findings | sampled | blinter | judged true | precision | notes |
|---|---|---|---|---|---|---|
| argument-prefix | 586 | 20 | 20 | 0 of 0 | 100% |  |
| blank-line-before-label | 356 | 20 | 20 | 0 of 0 | 100% |  |
| command-case | 204 | 20 | 20 | 0 of 0 | 100% |  |
| commented-block | 232 | 20 | 18 | 2 of 2 | 100% | 2 not reported by Blinter: undocumented 3-line `if` blocks, true by the rule |
| delayed-expansion-in-loop | 98 | 20 | 20 | 0 of 0 | 100% |  |
| documented-subroutine | 127 | 20 | 20 | 0 of 0 | 100% |  |
| feature-not-enabled | 139 | 20 | 18 | 2 of 2 | 100% | 2 not reported by Blinter: `!x!` after `endlocal & endlocal` closed the enabled scope, true |
| for-f-options | 92 | 20 | 20 | 0 of 0 | 100% |  |
| if-compare-wildcard | 44 | 20 | 19 | 1 of 1 | 100% | 1 not reported by Blinter: `%*` as the left operand next to `==` |
| missing-terminator | 454 | 20 | 20 | 0 of 0 | 100% |  |
| nested-for | 217 | 20 | 17 | 3 of 3 | 100% | 3 not reported by Blinter: loops that are nested, true by the rule |
| no-duplicate-label | 70 | 20 | 19 | 0 of 1 | 95% | 1 false: `: ; abort () {...}` in a shell/batch polyglot |
| no-execution-policy-bypass | 33 | 20 | 18 | 2 of 2 | 100% | 2 not reported by Blinter: the switch is on a continuation line, true |
| no-forced-registry-delete | 215 | 20 | 20 | 0 of 0 | 100% |  |
| no-if-not-errorlevel-zero | 25 | 20 | 20 | 0 of 0 | 100% |  |
| no-late-at-sign | 465 | 20 | 18 | 0 of 2 | 90% | 2 false: `@{` and `@"` of a PowerShell hash table or here-string in a polyglot |
| no-long-one-liner | 432 | 20 | 20 | 0 of 0 | 100% |  |
| no-magic-numbers | 1449 | 20 | 19 | 0 of 1 | 95% | 1 false: `127:ProfessionalS` in a data list |
| no-non-ascii | 39 | 20 | 20 | 0 of 0 | 100% |  |
| no-redundant-parens | 299 | 20 | 20 | 0 of 0 | 100% |  |
| no-shift-in-block | 86 | 20 | 20 | 0 of 0 | 100% |  |
| no-unconfirmed-delete-tree | 103 | 20 | 19 | 1 of 1 | 100% | 1 not reported by Blinter: `rmdir /s /q` on a path, true |
| parse-time-variable-in-block | 67 | 20 | 16 | 4 of 4 | 100% | 4 not reported by Blinter: `%errorlevel%` in `echo` and `exit /b` inside a block, true by the rule |
| quote-cd-variable | 37 | 20 | 20 | 0 of 0 | 100% |  |
| quote-if-operand | 527 | 20 | 20 | 0 of 0 | 100% |  |
| quote-if-operands-both | 176 | 20 | 20 | 0 of 0 | 100% |  |
| repeated-command | 43 | 20 | 20 | 0 of 0 | 100% |  |
| stderr-redirect-last | 30 | 20 | 20 | 0 of 0 | 100% |  |
| unchecked-command | 993 | 20 | 20 | 0 of 0 | 100% |  |
| unused-opener | 41 | 20 | 20 | 0 of 0 | 100% |  |
