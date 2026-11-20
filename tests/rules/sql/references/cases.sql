select * from t;
--     ^ unknown-columns
select a, b from t1 join t2 on t1.id = t2.id;
--     ^ unqualified-reference
--        ^ unqualified-reference
select t.a, b from t;
--          ^ inconsistent-reference
select a, b as a from t;
--             ^ duplicate-column-alias
select a, b from t group by a, 2;
--                 ^^^^^^^^^^^^^ group-order-style
with c as (select a from t) select * from c;
select x.a from (select * from t) as x;
