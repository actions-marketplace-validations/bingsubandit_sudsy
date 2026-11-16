select a from t where a = null and b <> null and c != NULL and null = d;
--                      ^^ null-comparison
--                                   ^^^ null-comparison
--                                                 ^^^ null-comparison
--                                                                  ^^ null-comparison
select a from t where a is null and b is not null and c = 'null';
select distinct a from t group by a;
--     ^^^^^^^^ distinct-group-by
select distinct a from t;
select a from t group by a;
select a as a, b as c, d.e as e from t;
--          ^ self-alias
select a from t as x join u as x on x.a = x.b;
--                             ^ duplicate-table-alias
select a from t join u on t.a = u.a join t on t.a = t.b;
--                                       ^ duplicate-table-alias
select a from t join u on t.a = u.a join v on t.a = v.b;
with a as (select 1), b as (select 2), c as (select * from a) select * from c;
--                    ^ unused-cte
with recursive r as (select 1 union all select n from r) select 2;
