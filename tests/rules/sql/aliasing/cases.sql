select 1, sum(a), a + 1, b, t.*, a::int, (a), a & b, case when a then 1 end, count(*) as n from t;
--     ^ expression-alias
--        ^^^^^^ expression-alias
--                ^^^^^ expression-alias
--                                            ^^^^^ expression-alias
--                                                   ^^^^^^^^^^^^^^^^^^^^^^ expression-alias
