SELECT a, B, COUNT(c), sum(d), NULL, true, cast(e as INT), cast(f as text)
--        ^ identifier-case
--                     ^^^ function-case
--                                   ^^^^ literal-case
--                                         ^^^^ function-case
--                                                ^^ keyword-case
--                                                         ^^^^ function-case
--                                                                ^^ keyword-case
--                                                                   ^^^^ type-case
from t
^^^^ keyword-case
WHERE a = 1;
