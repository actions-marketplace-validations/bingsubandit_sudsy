-- alter_sequence.sql
ALTER SEQUENCE foo INCREMENT BY 1;

ALTER SEQUENCE foo MAXVALUE 7 NO minvalue;

ALTER SEQUENCE foo NOCACHE CYCLE;

ALTER SEQUENCE foo NOORDER CACHE 5 NOCYCLE;

ALTER SEQUENCE foo ORDER;

-- alter_table.sql
ALTER TABLE x DROP COLUMN y;
ALTER TABLE x DROP y;
ALTER TABLE x DROP IF EXISTS y;
ALTER TABLE x DROP COLUMN IF EXISTS y;

-- alter_table_rename_to.sql
ALTER TABLE old_table_name RENAME TO new_table_name;

-- ansi_cast_with_whitespaces.sql
-- ansi_cast_with_whitespaces.sql
/* Several valid queries where there is whitespace surrounding the ANSI
cast operator (::) */

-- query from https://github.com/sqlfluff/sqlfluff/issues/2720
SELECT amount_of_honey :: FLOAT
FROM bear_inventory;


-- should be able to support an arbitrary amount of whitespace
SELECT amount_of_honey        ::        FLOAT
FROM bear_inventory;

SELECT amount_of_honey::    FLOAT
FROM bear_inventory;

SELECT amount_of_honey        ::FLOAT
FROM bear_inventory;

-- should support a wide variety of typecasts
SELECT amount_of_honey :: time
FROM bear_inventory;

SELECT amount_of_honey :: text
FROM bear_inventory;

SELECT amount_of_honey     :: VARCHAR( 512 )
FROM bear_inventory;

SELECT amount_of_honey ::        TIMESTAMPTZ
FROM bear_inventory;

SELECT amount_of_honey    ::        TIMESTAMP WITHOUT TIME ZONE
FROM bear_inventory;

-- should support casts with arbitrary amount of whitespace in join statements
SELECT
    bi.amount_of_honey
FROM bear_inventory bi
LEFT JOIN favorite_cola fc
    ON fc.bear_id ::   VARCHAR(512) = bi.bear_id    ::VARCHAR(512)
WHERE fc.favorite_cola = 'RC Cola';

-- arithmetic_a.sql
SELECT 1 + (2 * 3) >= 4 + 6+13 as val;

SELECT 1 + ~(~2 * 3) >= 4 + ~6+13 as val;

SELECT -1;

SELECT -1 + 5;

SELECT ~1;

SELECT -1 + ~5;

SELECT 4 & ~8 | 16;

SELECT 8 + ~(3);

SELECT 8 | ~ ~ ~4;

SELECT 1 * -(5);

SELECT 1 * -5;

SELECT 1 * - - - 5;

SELECT 1 * - - - (5);

SELECT 1 * + + (5);

SELECT 1 * - - - func(5);

SELECT 1 * ~ ~ ~ func(5);

SELECT 1 * +(5);

SELECT 1 * +5;

SELECT 1 * + + 5;

SELECT FALSE AND NOT (TRUE);

SELECT FALSE AND NOT NOT NOT (TRUE); -- parses middle NOT as column ref

SELECT FALSE AND NOT (TRUE);

SELECT FALSE AND NOT func(5);

SELECT 'abc' LIKE - - 5; -- PG can parse this ok, and then fail due to data type mismatch

SELECT 'abc' LIKE ~ ~ 5; -- PG can parse this ok, and then fail due to data type mismatch
;

-- bracket_in_comment.sql
select a
/*
)
*/
from b
;

-- bracketed_statement.sql
(SELECT 1);

((SELECT 1));

(((SELECT 1)));

-- comments.sql
-- This is a comment

/* So is this */

/* This is a
    multiple line comment */
;

-- commit.sql
commit
;

-- commit_and_no_chain.sql
commit and no chain
;

-- commit_work.sql
commit work
;

-- commit_work_and_no_chain.sql
commit work and no chain
;

-- create_cast.sql
CREATE CAST (int AS bool) WITH FUNCTION fname;

CREATE CAST (int AS bool) WITH FUNCTION fname AS ASSIGNMENT;

CREATE CAST (int AS bool) WITH FUNCTION fname();

CREATE CAST (int AS bool) WITH FUNCTION fname(bool);

CREATE CAST (int AS bool) WITH FUNCTION sch.fname(int, bool) AS ASSIGNMENT;

CREATE CAST (udt_1 AS udt_2) WITH FUNCTION fname(udt_1, udt_2) FOR udt_3;

CREATE CAST (sch.udt_1 AS sch.udt_2) WITH FUNCTION sch.fname(sch.udt_1, sch.udt_2) FOR sch.udt_3;

CREATE CAST (int AS bool) WITH ROUTINE fname();

CREATE CAST (int AS bool) WITH PROCEDURE fname();

CREATE CAST (int AS bool) WITH METHOD fname();

CREATE CAST (int AS bool) WITH INSTANCE METHOD fname();

CREATE CAST (int AS bool) WITH STATIC METHOD fname();

CREATE CAST (int AS bool) WITH CONSTRUCTOR METHOD fname();

-- create_database_a.sql
create database my_database
;

-- create_database_if_not_exists.sql
create database if not exists my_database
;

-- create_function.sql
CREATE FUNCTION add(integer, integer) RETURNS integer
    AS 'select $1 + $2;'
    LANGUAGE SQL;

DROP FUNCTION add;

-- create_function_no_args.sql
CREATE FUNCTION add() RETURNS integer
    AS 'select $1 + $2;'
    LANGUAGE SQL;

-- create_index_if_not_exists.sql
CREATE INDEX IF NOT EXISTS transaction_updated ON transaction_master (last_updated);

-- create_index_simple.sql
CREATE INDEX transaction_updated ON transaction_master(last_updated);
CREATE UNIQUE INDEX transaction_updated ON transaction_master(last_updated);

-- create_model_options.sql
CREATE OR REPLACE MODEL model3
OPTIONS (
    MODEL_TYPE='LOGISTIC_REG',
    AUTO_CLASS_WEIGHTS=TRUE,
    INPUT_LABEL_COLS = ['label_str']
)
AS
SELECT
    a,
    b
FROM
    table1
WHERE
    training = 1
;

-- create_role.sql
CREATE ROLE foo_role
;

-- create_schema_a.sql
create schema my_schema
;

-- create_schema_if_not_exists.sql
create schema if not exists my_schema
;

-- create_sequence.sql
CREATE SEQUENCE foo;

CREATE SEQUENCE foo INCREMENT BY 3;

CREATE SEQUENCE foo MINVALUE 5 NO MAXVALUE;

CREATE SEQUENCE foo NO MINVALUE MAXVALUE 12;

CREATE SEQUENCE foo INCREMENT BY 5 START WITH 8 CACHE 4;

CREATE SEQUENCE foo NOCACHE;

CREATE SEQUENCE foo NOCYCLE ORDER;

CREATE SEQUENCE foo NOORDER;

-- create_table.sql
-- Test various forms of quoted data types
CREATE TABLE foo (
    pk int PRIMARY KEY,
    quoted_name "custom udt",
    qualified_name sch.qualified,
    quoted_qualified "my schema".qualified,
    more_quoted "my schema"."custom udt",
    quoted_udt sch."custom udt"
);

-- create_table_a_c1_c2.sql
create table table1 (c1 SMALLINT, c2 DATE)
;

-- create_table_a_column_constraints.sql
create table table1 (
    c1 INT NOT NULL,
    c2 INT NULL DEFAULT 1,
    c3 INT PRIMARY KEY,
    c4 INT UNIQUE,
    c5 INT REFERENCES table2,
    c6 INT REFERENCES table2 (c6_other),
    c6 INT REFERENCES table2 (c6_other) MATCH FULL,
    c6 INT REFERENCES table2 (c6_other) MATCH PARTIAL,
    c6 INT REFERENCES table2 (c6_other) MATCH SIMPLE,
    c6 INT REFERENCES table2 (c6_other) ON DELETE NO ACTION,
    c6 INT REFERENCES table2 (c6_other) ON UPDATE SET NULL,
    c6 INT REFERENCES table2 (c6_other) ON DELETE RESTRICT ON UPDATE CASCADE,
    c7 INT NOT NULL DEFAULT 1 UNIQUE REFERENCES table3 (c7_other),
    c8 INT NOT NULL DEFAULT 1::INT
)
;

-- create_table_a_pk_unique_fk_constraints.sql
create table table1 (
    c1 INT,
    c2 INT,
    c3 INT,
    PRIMARY KEY (c1),
    UNIQUE (c2, c3),
    FOREIGN KEY (c2, c3) REFERENCES table2 (c2_, c3_)
)
;

-- create_table_as.sql
CREATE OR REPLACE TABLE  t2 AS
SELECT
  a,
  b
FROM t1
;

-- create_table_as_select_cte_no_parentheses.sql
CREATE TABLE final_rows AS
WITH source_table AS
(
    SELECT * FROM source_data
)
SELECT * FROM source_table
;

-- create_table_as_select_cte_parentheses.sql
CREATE TABLE final_rows AS
(
    WITH source_table AS
    (
        SELECT * FROM source_data
    )
    SELECT * FROM source_table
)
;

-- create_table_auto_increment.sql
CREATE TABLE a (
    id INT AUTO_INCREMENT
)
;

-- create_table_column_comment.sql
CREATE TABLE a (
    id VARCHAR(100) COMMENT 'Column comment'
)
;

-- create_table_column_constraint.sql
CREATE TABLE users (
    username TEXT,
    age INT CHECK(age > 18)
);

CREATE TABLE users (
    username TEXT,
    age INT CHECK(age IS NOT NULL)
);

CREATE TABLE Persons (
    ID int NOT NULL,
    LastName varchar(255) NOT NULL,
    FirstName varchar(255),
    Age int,
    City varchar(255),
    CONSTRAINT CHK_Person CHECK (Age>=18 AND City='Sandnes')
);

-- create_table_constraint_default.sql
BEGIN TRANSACTION;
CREATE TABLE IF NOT EXISTS "tbl" ( "col" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP );
COMMIT;

-- create_table_constraint_reference_option.sql
CREATE TABLE b (
    b INT NOT NULL,
    c INT NOT NULL,
    d INT NOT NULL,
    CONSTRAINT c_b FOREIGN KEY (b) REFERENCES a(b) ON DELETE RESTRICT ON UPDATE NO ACTION,
    CONSTRAINT c_d FOREIGN KEY (d) REFERENCES a(d) ON UPDATE CASCADE ON DELETE SET NULL,
    CONSTRAINT c_c FOREIGN KEY (c) REFERENCES a(c) ON DELETE SET DEFAULT
)
;

-- create_table_default_function.sql
CREATE TABLE a (
    ts TIMESTAMP DEFAULT GETDATE()
)
;

-- create_table_double_precision.sql
CREATE TABLE test ( angle double precision );

-- create_table_table_comment.sql
CREATE TABLE a (
    id VARCHAR(100)
)
COMMENT 'Table comment'
;

-- create_table_varchar.sql
CREATE TABLE a (
    id VARCHAR(100)
)
;

-- create_trigger.sql
CREATE TRIGGER foo BEFORE INSERT ON bar EXECUTE PROCEDURE proc(args);

CREATE TRIGGER foo BEFORE INSERT on bar EXECUTE PROCEDURE proc(args);

CREATE TRIGGER foo AFTER UPDATE OF bar, baz ON bar EXECUTE PROCEDURE proc(args);

CREATE TRIGGER foo INSTEAD OF DELETE ON bar FROM baz DEFERRABLE INITIALLY IMMEDIATE FOR EACH ROW EXECUTE PROCEDURE proc(args);

CREATE TRIGGER foo BEFORE INSERT ON bar WHEN (a=b) EXECUTE PROCEDURE proc(args);

-- create_user.sql
CREATE USER foo_user
;

-- create_view_a.sql
CREATE VIEW a
AS
    SELECT
        c
    FROM table1
    INNER JOIN table2 ON (table1.id = table2.id);

CREATE OR REPLACE VIEW vw_appt_latest AS (
  WITH most_current as (
      SELECT
            da.*
      FROM dim_appt da
      WHERE da.current_appt_id IS NULL
      )
  SELECT * from most_current
);

-- create_view_if_not_exists.sql
CREATE VIEW IF NOT EXISTS a
AS
    SELECT
        c
    FROM table1
    INNER JOIN table2 ON (table1.id = table2.id)
;

-- create_zero_argument_function.sql
CREATE OR REPLACE FUNCTION a() RETURNS integer
AS
'
    SELECT 1;
'
LANGUAGE SQL;

CREATE FUNCTION a() RETURNS integer
AS
'
    SELECT 1;
'
LANGUAGE SQL;

-- date_part_function_arguments.sql
SELECT DATEADD(x, day) FROM t;

SELECT DATEADD('year', day, month) FROM t;

-- delete_from.sql
DELETE FROM table_name
WHERE a > 0;

-- delete_truncate.sql
-- Indented DELETE and TRUNCATE targets.
DELETE FROM
    tbl
WHERE x = 1;

TRUNCATE TABLE
    tbl;

-- describe_table.sql
describe table "my_table";

-- double_dot.sql
-- Snowflake Double-Dot Notation
-- https://docs.snowflake.com/en/sql-reference/name-resolution.html#resolution-when-schema-omitted-double-dot-notation
SELECT *
FROM my_database..my_table
;

-- double_quote_escapes.sql
select
    1 as foo,
    2 as "foo",
    3 as """foo""",
    4 as """""foo""""",
    bar,
    "bar",
    """bar""",
    """""bar"""""
from """""a"""""."""""b"""""."""""c"""""
;

-- drop_cast.sql
DROP CAST (int AS bool);

DROP CAST (int AS bool) RESTRICT;

DROP CAST (int AS bool) CASCADE;

DROP CAST (udt_1 AS udt_2);

DROP CAST (sch.udt_1 AS sch.udt_2);

-- drop_index_if_exists.sql
DROP INDEX IF EXISTS transaction_updated;

-- drop_index_simple.sql
DROP INDEX transaction_updated;

-- drop_model.sql
DROP MODEL IF EXISTS model3
;

-- drop_schema.sql
drop schema my_schema;
drop schema my_schema cascade;
drop schema my_schema restrict;
drop schema if exists my_schema;
drop schema if exists my_schema cascade;
drop schema if exists my_schema restrict;

-- drop_sequence.sql
DROP SEQUENCE foo;

DROP SEQUENCE foo.foo;

-- drop_table_a.sql
drop table a
;

-- drop_table_a_cascade.sql
drop table a cascade
;

-- drop_table_a_restrict.sql
drop table a restrict
;

-- drop_table_if_exists_a.sql
drop table if exists a
;

-- drop_trigger.sql
DROP TRIGGER foo;

-- drop_type.sql
DROP TYPE typename;

DROP TYPE IF EXISTS typename;

DROP TYPE typename CASCADE;

DROP TYPE typename RESTRICT;

-- drop_view_a.sql
drop view a
;

-- drop_view_a_cascade.sql
drop view a cascade
;

-- drop_view_a_restrict.sql
drop view a restrict
;

-- empty_file.sql

;

-- escape.sql
SELECT *
FROM x
WHERE z LIKE '^_f' ESCAPE '^'
;

-- escaped_quotes.sql
select case when "Spec\"s 23" like 'Spec\'s%' then 'boop' end as field;

select 'This shouldn''t fail' as success;

-- expression_recursion.sql
 -- This test checks for recursion errors. If the expression
 -- is not parsed correctly it can lead to very deep recursion.

 -- If this test is failing, then check the structure of expression
 -- parsing.

 select
        1
    from
        test_table
    where
        test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%' --5
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%' -- 10
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%' -- 15
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%' -- 20
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%' --30
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%'
        or test_table.string_field like 'some string%' -- 40
;

-- expression_recursion_2.sql
 -- This test checks for recursion errors. If the expression
 -- is not parsed correctly it can lead to very deep recursion.

 -- If this test is failing, then check the structure of expression
 -- parsing.

 SELECT * FROM t WHERE a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b AND a < b
;

-- from_fetch.sql
SELECT *
FROM counter
FETCH FIRST 10 ROWS ONLY
;

-- functions_a.sql
SELECT
    DATE(t), ROUND(b, 2),
    LEFT(right(s, 5), LEN(s + 6)) as compound
FROM tbl_b;

SELECT _custom_function(5) as test_column;

-- functions_b.sql
-- Thanks @mrshu for this query, it tests nested functions
SELECT
    SPLIT(LOWER(text), ' ') AS text
FROM "database"."sample_table"
;

-- grant_all_on_mytable_to_role.sql
grant all on mytable to public
;

-- grant_all_on_table_mytable_to_role.sql
grant all on table mytable to myrole
;

-- grant_all_privileges_on_mytable_to_role.sql
grant all privileges on mytable to myrole
;

-- grant_multiple_tables.sql
GRANT INSERT ON my_table, my_table2 TO public;
GRANT INSERT ON my_table, my_table2 TO "public";
GRANT INSERT, UPDATE ON my_table, my_table2 TO public;
GRANT INSERT, UPDATE ON my_table, my_table2 TO "public";
GRANT INSERT, UPDATE, DELETE ON my_table, my_table2 TO public;
GRANT INSERT, UPDATE, DELETE ON my_table, my_table2 TO "public";
GRANT INSERT, UPDATE, DELETE, SELECT ON my_table, my_table2 TO public;
GRANT INSERT, UPDATE, DELETE, SELECT ON my_table, my_table2 TO "public";

-- grant_select_col1_col2_update_col1_on_mytable_to_public.sql
grant select (col1, col2), update (col1) on mytable to public
;

-- grant_select_on_mytable_to_public.sql
grant select on mytable to public
;

-- grant_select_on_mytable_to_public_with_grant_option.sql
grant select on mytable to public with grant option
;

-- grant_select_update_insert_on_mytable_to_public.sql
grant select, update, insert on mytable to public
;

-- grant_update_on_all_tables_in_schema_a_to_public.sql
grant update on all tables in schema a to public
;

-- grant_usage_on_all_file_formats_masking_policies.sql
GRANT USAGE ON ALL FILE FORMATS IN SCHEMA s TO r;
GRANT USAGE ON ALL MASKING POLICIES IN SCHEMA s TO r;

-- group_by_fetch.sql
SELECT
    status
FROM
    orders
GROUP BY
    status
FETCH FIRST 3 ROWS ONLY
;

-- having_fetch.sql
SELECT
	house_id,
	COUNT (person_id)
FROM
	persons
GROUP BY
	house_id
HAVING
	COUNT (person_id) > 10
FETCH FIRST 30 ROWS ONLY
;

-- insert_a.sql
INSERT into tbl_b (col1) values (123);

INSERT INTO tbl_c (
    SELECT *
    FROM table1
);

-- insert_default_values.sql
INSERT INTO mytable
DEFAULT VALUES;

-- insert_indented.sql
-- Indented INSERT target with nested VALUES rows.
INSERT INTO
    tbl (
        col1,
        col2
    )
VALUES
    (1, 2),
    (3, 4);

INSERT INTO tbl (col1) VALUES (1);

-- insert_using_subquery.sql
INSERT INTO foo SELECT 0 AS bar;

INSERT INTO foo (SELECT 1 AS bar);

INSERT INTO foo ((SELECT 1 AS bar));

-- insert_with_statement.sql
INSERT INTO table2 (column1, column2, column3)
WITH mycte AS (
    SELECT
        foo,
        bar
    FROM mytable1
)

SELECT
    foo,
    bar,
    baz
FROM mycte;

-- is_distinct_from.sql
-- IS [NOT] DISTINCT FROM in SELECT clause (previously failed to parse)
SELECT NULL IS DISTINCT FROM TRUE;
SELECT NULL IS NOT DISTINCT FROM TRUE;

-- IS DISTINCT FROM with column references and FROM clause
SELECT a IS DISTINCT FROM b FROM t;
SELECT a IS NOT DISTINCT FROM b FROM t;

-- IS DISTINCT FROM with SELECT DISTINCT modifier
SELECT DISTINCT a IS DISTINCT FROM b FROM t;

-- IS DISTINCT FROM with multiple select elements
SELECT a, b IS DISTINCT FROM c, d FROM t;

-- IS [NOT] DISTINCT FROM with inline and block comments
SELECT a IS -- inline comment
DISTINCT FROM b FROM t;
SELECT a IS /* block comment */ NOT DISTINCT FROM b FROM t;

-- IS DISTINCT FROM in WHERE clause (always worked)
SELECT a FROM t WHERE a IS DISTINCT FROM b;
SELECT a FROM t WHERE a IS NOT DISTINCT FROM b;

-- like_operators.sql
-- https://github.com/sqlfluff/sqlfluff/issues/828
-- https://github.com/sqlfluff/sqlfluff/issues/842
-- https://www.postgresql.org/docs/9.0/functions-matching.html#FUNCTIONS-LIKE
SELECT *
FROM my_tbl
WHERE a !~ '[a-z]'
AND d !~~* '[a-z]'
AND b LIKE 'Spec\'s%'
AND c !~* '^([0-9]){1,}(\.)([0-9]{1,})$'
;

-- modulo.sql
SELECT
    CASE
        WHEN (year_number % 400 = 0)
            OR (year_number % 4 = 0 AND year_number % 100 != 0)
            THEN TRUE ELSE FALSE
    END AS is_leap_year
FROM mytable
;

-- multi_statement_a.sql
select a from tbl1  /*comment here*/ ;  /*and here*/  select b from tbl2;   -- trailling ending comment
;

-- multi_statement_b.sql
# COMMENT
-- Another Comment
Select A from Sys.dual where a
-- inline comment
in  ('RED',  /* Inline */  'GREEN','BLUE');
select * from tbl_b; # as another comment
insert into sch.tbl_b
    (col1)
values (123);
with tmp as (
    select * from  blah
)
select a, b from tmp;
# And that's the end
;

-- multi_statement_c.sql
select
    *
from boo;
WITH blah AS (select x,y,4.567 FROM foo) select z, y, x from blah;

-- multiple_semicolons.sql
-- Leading double semicolons before first statement
;;SELECT col1 FROM tbl1;

-- Statement with trailing double semicolons
SELECT col2 FROM tbl2;;

-- Multiple semicolons separating statements
SELECT col3 FROM tbl3;;;;SELECT col4 FROM tbl4;

-- Mixed whitespace and semicolons
; ; ;SELECT col5 FROM tbl5;   ;  ;

-- Many semicolons at end
SELECT col6 FROM tbl6;;;;;;

-- Many semicolons at end
SELECT col6 FROM tbl6;;;;;;

-- naked_identifiers.sql
-- A valid identifier is alphanumeric and contains at least one letter.
select "a" as 0_is_a_legal_identifier;
select "a" as 00_is_a_legal_identifier;
select 123_is_a_legal_identifier.456_is_a_legal_identifier;
select "a" as 0is_a_legal_identifier;
select _s00.45_is_a_legal_identifier from sdf9_._234awdf;
select "a" as is_a_legal_identifier0;

-- non_breaking_space.sql
#space before from is non-breaking space
SELECT a,b, c  from sch."blah"
;

-- numeric_literal.sql
SELECT
    1,
    1.2,
    1.,
    .2,
    1e3,
    1E3,
    1.2e+3,
    1.2E+3,
    1.e-3,
    1.E-3,
    .2e3,
    .2E3
;

-- obevo.sql
//// CHANGE name=alter1

ALTER TABLE table1 ADD COLUMN colY NUMBER;

ALTER TABLE table1 ADD COLUMN colZ NUMBER;

//// METADATA test
//// CHANGE name="init"
CREATE TABLE OrigTable (
    Field1 int,
    Field2 int
)
//// CHANGE name="dropOld" DROP_TABLE dependencies="OldToNewTableMigration.migration"
;

-- revoke_select_on_table_a_from_group_b.sql
revoke select on table a from group b
;

-- rollback.sql
rollback
;

-- rollback_and_no_chain.sql
rollback and no chain
;

-- rollback_work.sql
rollback work
;

-- rollback_work_and_no_chain.sql
rollback work and no chain
;

-- select_a.sql
select a,b, c from sch."blah"
;

-- select_a_and_not_b.sql
-- https://github.com/sqlfluff/sqlfluff/issues/827

SELECT
  a AND NOT i.b
FROM i
;

-- select_b.sql
select * from foo JOIN bar ON (foo.a = bar.a)
;

-- select_bitwise_operators.sql
select
    c1 & c2,
    c3 | c4,
    c5 ^ c6,
    c7 << c8,
    c9 >> c10
;

-- select_c.sql
-- Thanks @mrshu for this query, it tests functions and order by
SELECT
    col_a,
    col_b,
    date_col_a,
    date_col_b
FROM "database"."sample_table"
WHERE
    DATE(date_col_b) >= current_date
    AND length(col_a) = 4
ORDER BY date_col_a DESC NULLS LAST
;

-- select_case_a.sql
SELECT
    CASE WHEN 1 = 2 THEN 3
    WHEN 4 > 3 THEN 5 + 2
    WHEN some_var IN (1,2,3) then "nothing"
    ELSE "boo"
    END as a_case_statement
FROM boo
;

-- select_case_b.sql
SELECT
    CASE some_var
    WHEN 'hello' THEN 3
    WHEN 'hi' THEN 12
    ELSE 0
    END as a_case_statement
FROM boo
;

-- select_case_c.sql
select
    col0,
    case
        when col1
        then col2
        else col3
    end::text as mycol
from table1;

select
    col0,
    case
        when col1
        then col2
        else col3
    end::int::float as mycol
from table1;

select
    col0,
    cast(case
        when col1
        then col2
        else col3
    end as text) as mycol
from table1;

SELECT
    column1,
    CASE
        WHEN 'test' THEN 12
        ELSE 0
    END >= 0
FROM boo;

-- select_cross_join.sql
-- https://github.com/sqlfluff/sqlfluff/issues/871
WITH constants AS (
    SELECT 8760 AS hours_per_year
)

SELECT
    table1.name,
    foo.name,
    foo.value * constants.hours_per_year AS some_value
FROM table1
CROSS JOIN
    constants
JOIN table2 AS foo USING (id)
;

-- select_d.sql
SELECT
    col_a,
    col_b
FROM some_table
WHERE col_a IS NOT NULL
AND col_b NOT IN (SELECT c FROM another_table)
;

-- select_e.sql
-- Union expressions
SELECT
    col_a as foo
FROM some_table

UNION

SELECT
    col_b as foo
FROM another_table

UNION ALL

SELECT
    col_c as foo
FROM this_other_table

INTERSECT

SELECT
    col_d as foo
FROM the_last_table
;

-- select_except.sql
SELECT 1
EXCEPT
SELECT 2
;

SELECT 1
EXCEPT ALL
SELECT 2
;

-- select_f.sql
-- Test the parser on complex maths
SELECT
    COS(2*ACOS(-1)*2*y/53) AS c2
FROM
    t
;

-- select_fetch.sql
SELECT
    EMPLOYEE.EMPNO
FROM
    EMPLOYEE
FETCH FIRST 3 ROWS ONLY
;

-- select_fn_square_bracket_array_parameter.sql
SELECT myfn([1, 2, 3])
;

-- select_from_where_overlaps.sql
SELECT column_name FROM table_name WHERE period1 OVERLAPS period2;

-- select_function_in_group_by.sql
SELECT
    COALESCE(id, 1)
FROM some_table
GROUP BY COALESCE(id, 1)
;

-- select_g.sql
-- More complex select clause without from clause
SELECT
    NULL::INT AS user_id,
    NULL::INT AS is_paid
LIMIT 0
;

-- select_g_fetch.sql
-- More complex select clause without from clause
SELECT
    NULL::INT AS user_id,
    NULL::INT AS is_paid
FETCH FIRST 0 ROWS ONLY
;

-- select_h.sql
SELECT
    DATE(zendesk.created_at, 'America/New_York') AS date,
    COUNT(
        CASE
        WHEN zendesk.support_team IN ('tech support', 'taskus', 'onc') THEN 1
        END
    ) AS tech_support
FROM
    zendesk
;

-- select_in_multiline_comment.sql
with a as (

    select *
    from table_a

),

/*
    select
*/

b as (

    select *
    from a

)

select *
from b
;

-- select_intersect.sql
SELECT 1
INTERSECT
SELECT 2
;

SELECT 1
INTERSECT ALL
SELECT 2
;

-- select_j.sql
-- Aliasing without AS
-- https://github.com/sqlfluff/sqlfluff/issues/149
SELECT
    (POW(sd2,2) + POW(sd3,2) + POW(sd4,2) + POW(sd4,2)) w1
FROM
    dat;
-- Another Aliasing without AS
SELECT
    CASE
      WHEN order_month = max_month THEN 1
    ELSE
    0
  END
    churn
;

-- select_l.sql
-- Nested scalar query
-- https://github.com/sqlfluff/sqlfluff/issues/147
SELECT
  a
FROM
  dat
WHERE
  c >= (SELECT 1)
;

-- select_m.sql
-- On clause without brackets
-- https://github.com/sqlfluff/sqlfluff/issues/146
SELECT
    a
FROM
    zendesk
LEFT JOIN
    ticket
ON
    zendesk.ticket_id = ticket.id;

SELECT
    low_user_counts
FROM
    acceptable_buckets
JOIN
    small_buckets
ON
    (business_type = low_business_type)
    AND (business_type = low_business_type
        OR size_label = low_size_label);

-- select_many_join.sql
SELECT *
FROM a as foo JOIN b JOIN c as foobar JOIN d, e as bar JOIN f JOIN g('blah') as tbl_func JOIN h, baz as buzz;


SELECT
 c.f1 as f1
 , co.f2 as f2
 , po.f3 as f3
 , c2c.f4 as f4
 , c_ph.f5 as f5
FROM t1 AS c
LEFT JOIN t2 AS co
 ON c.f1 = co.f1
LEFT JOIN t3 AS po
 ON c.f1 = po.f1
LEFT JOIN (
 SELECT t._tmp as _tmp
 FROM (SELECT * FROM t4) AS t
) AS l_ccc
 ON c.f1 = l_ccc._tmp
LEFT JOIN t5 AS cc
 ON l_ccc._tmp = cc.f1
LEFT JOIN (
     (
         SELECT t._tmp AS _tmp
         FROM (SELECT * FROM t6) AS t
     ) AS l_c2c_c
     LEFT JOIN (
         SELECT a1._tmp AS _tmp
           , h.id
           , h.f1
         FROM (
           SELECT t.id
             , t.f4

           FROM (SELECT * FROM t7) AS t) AS h
       LEFT JOIN (SELECT * FROM t8) AS a1
                 ON a1.id = h.id
     ) AS c2c
             ON l_c2c_c._tmp = c2c.id
)
ON c.f1 = l_c2c_c._tmp
LEFT JOIN t9 AS c_ph
 ON c.f1 = c_ph.f1;

-- select_multiple_named_windows.sql
SELECT item, purchases, category, LAST_VALUE(item)
  OVER (d) AS most_popular
FROM Produce
WINDOW
  a AS (PARTITION BY category),
  b AS (a ORDER BY purchases),
  c AS (b ROWS BETWEEN 2 PRECEDING AND 2 FOLLOWING),
  d AS (c)
;

-- select_n.sql
-- Full Join
-- https://github.com/sqlfluff/sqlfluff/issues/144
SELECT
    exists_left.business_type AS business_type_left,
    exists_right.business_type AS business_type_right
FROM
    benchmark_summaries AS exists_left
FULL JOIN
    business_types AS exists_right
ON
    exists_left.business_type = exists_right.business_type
;

-- select_named_window.sql
SELECT AVG(col) OVER(win) FROM table WINDOW win AS (PARTITION BY date)
;

-- select_named_window_no_parentheses.sql
SELECT
    NTH_VALUE(bar, 1) OVER w1 AS baz
FROM t
WINDOW w1 AS (
    PARTITION BY
        x,
        y,
        z
    ORDER BY abc DESC
)
;

-- select_named_window_with_parentheses.sql
SELECT
    NTH_VALUE(bar, 1) OVER(w1) AS baz,
    NTH_VALUE(bar, 1) OVER() AS foo
FROM t
WINDOW w1 AS (
    PARTITION BY
        x,
        y,
        z
    ORDER BY abc DESC
)
;

-- select_named_windows_each_window_specification.sql
SELECT item, purchases, category, LAST_VALUE(item)
  OVER (c) AS most_popular
FROM Produce
WINDOW
  a AS (PARTITION BY category),
  b AS (ORDER BY purchases),
  c AS (ROWS BETWEEN 2 PRECEDING AND 2 FOLLOWING)
;

-- select_natural_join.sql
SELECT *
FROM table1
NATURAL JOIN table2;

SELECT *
FROM table1
NATURAL INNER JOIN table2;

SELECT *
FROM table1
NATURAL LEFT JOIN table2;

SELECT *
FROM table1
NATURAL LEFT OUTER JOIN table2;

SELECT *
FROM table1
NATURAL RIGHT JOIN table2;

SELECT *
FROM table1
NATURAL RIGHT OUTER JOIN table2;

SELECT *
FROM table1
NATURAL FULL JOIN table2;

SELECT *
FROM table1
NATURAL FULL OUTER JOIN table2;

-- select_nested_join.sql
-- query with no brackets
select
    orders.order_id AS order_id,
    customers.email AS email
from
    orders
    join customers on(
        (
            customers.customer_id = orders.customer_id
        )
    )
where
    (customers.email = 'sample@gmail.com')
group by
    orders.order_id,
    customers.email
order by
    orders.order_id;

-- nested bracketed up to 1 levels
select
    orders.order_id AS order_id,
    customers.email AS email
from
    (
        orders
        join customers on(
            (
                customers.customer_id = orders.customer_id
            )
        )
    )
where
    (customers.email = 'sample@gmail.com')
group by
    orders.order_id,
    customers.email
order by
    orders.order_id;

-- nested bracketed up to 2 levels
select
    orders.order_id AS order_id,
    customers.email AS email
from
    (
        (
            orders
            join customers on(
                (
                    customers.customer_id = orders.customer_id
                )
            )
        )
        join products on(
            (products.product_id = orders.product_id)
        )
    )
where
    (customers.email = 'sample@gmail.com')
group by
    orders.order_id,
    customers.email
order by
    orders.order_id;

-- nested bracketed up to 3 levels
select
    orders.order_id AS order_id,
    customers.email AS email
from
    (
        (
            (
                orders
                join customers on(
                    (
                        customers.customer_id = orders.customer_id
                    )
                )
            )
            join products on(
                (products.product_id = orders.product_id)
            )
        )
        join random on(
            (random.product_id = products.product_id)
        )
    )
where
    (customers.email = 'sample@gmail.com')
group by
    orders.order_id,
    customers.email
order by
    orders.order_id;

-- redundant brackets around a join used as a join target
-- https://github.com/sqlfluff/sqlfluff/issues/8382
select 1
from a
left join ((b inner join c on true)) on true;

-- three redundant layers, to pin that the depth is not capped
select 1
from a
left join (((b inner join c on true))) on true;

-- a join after the inner bracket
select 1
from a
left join ((b inner join c on true) left join d on true) on true;

-- redundant brackets in the FROM clause, which parsed before #8382 and must
-- keep the tree they had
select 1
from (((a inner join b on true)));

-- four layers
select 1
from a
left join ((((b inner join c on true)))) on true;

-- the alias inside the brackets is still the join target's alias
select a.x
from tbl_a as a
inner join ((tbl_b as b inner join tbl_c as c on b.id = c.id)) on a.id = b.id;

-- join forms other than LEFT ... ON
select 1
from a
cross join ((b inner join c on true));

select 1
from a
natural join ((b inner join c on true));

select 1
from a
left join ((b inner join c on true)) using (x);

select 1
from a
full outer join ((b inner join c on true)) on true;

-- select_numeric_literal_exponential_format.sql
SELECT
    1e-9,
    -12.345e12,
    .0123E-6,
    25,
    +6.34,
    0.5,
    25e-03,
    -1,
    25,
    -col1,
    -+-col2
;

-- select_o.sql
-- Between and Not Between
-- https://github.com/sqlfluff/sqlfluff/issues/142
-- https://github.com/sqlfluff/sqlfluff/issues/478
-- https://github.com/sqlfluff/sqlfluff/issues/2845
SELECT
    business_type
FROM
    benchmark_summaries
WHERE
    avg_click_rate NOT BETWEEN 0 and 1 + 1 + some_value
    AND some_other_thing BETWEEN 0 - 1 * another_value and 1
    AND another_thing BETWEEN -another_value and 0
;

-- select_offset.sql
SELECT * FROM test OFFSET 10 ROWS FETCH FIRST 10 ROWS ONLY;

SELECT Name, Address FROM Employees ORDER BY Salary OFFSET 2 ROWS FETCH FIRST 2 ROWS ONLY;

-- select_order_fetch.sql
SELECT
    EMPLOYEE.EMPNO
FROM
    EMPLOYEE
ORDER BY
    SALARY DESC
FETCH FIRST 3 ROWS ONLY
;

-- select_overlaps.sql
SELECT (DATE '2001-02-16', DATE '2001-12-21')
OVERLAPS (DATE '2001-10-30', DATE '2002-10-30');

-- select_p.sql
-- Case and Extract Expressions
-- https://github.com/sqlfluff/sqlfluff/issues/143
SELECT
    CAST(25.65 AS int),
    SAFE_CAST(NULL AS STRING) AS age_label,
    EXTRACT(day FROM end_time) AS day
FROM
    benchmark_with_performance
;

-- select_q.sql
-- Unexpected Join Fail
-- https://github.com/sqlfluff/sqlfluff/issues/163
SELECT
    data.id
FROM
    data
JOIN
    data_max
ON
    data.event_id = data_max.event_id
LEFT JOIN
    "other_db"."other_data" AS od
ON
    od.fid = data_max.fid
;

-- select_r.sql
-- Union All in a With
-- https://github.com/sqlfluff/sqlfluff/issues/162
WITH result AS (
    SELECT
        customer
    FROM sales_eu AS s
    UNION ALL
    SELECT
        customer
    FROM sales_us AS s2
)

SELECT * FROM result
;

-- select_right.sql
SELECT column_name
FROM table1
RIGHT JOIN table2
ON table1.column_name = table2.column_name
;

-- select_s.sql
-- Array notation (BigQuery and Postgres)
-- https://github.com/sqlfluff/sqlfluff/issues/59
SELECT
  user_id,
  list_id,
  (count_18_24 * bits[OFFSET(0)] + count_25_34 * bits[OFFSET(1)] +
   count_35_44 * bits[OFFSET(2)] + count_45_54 * bits[OFFSET(3)] +
   count_55_64 * bits[OFFSET(4)] + count_65_plus * bits[OFFSET(5)]) / audience_size AS relative_abundance
FROM
    gcp_project.dataset.audience_counts_gender_age
;

-- select_sample_bernoulli_10.sql
SELECT *
FROM foo TABLESAMPLE BERNOULLI (10)
;

-- select_sample_bernoulli_10_aliased.sql
SELECT f.colname
FROM foo AS f TABLESAMPLE BERNOULLI (10)
;

-- select_sample_bernoulli_10_repeatable.sql
SELECT *
FROM foo TABLESAMPLE BERNOULLI (10) REPEATABLE (100)
;

-- select_sample_bernoulli_order_limit.sql
SELECT col1, col2
FROM foo TABLESAMPLE BERNOULLI (10)
ORDER BY col1
LIMIT 100
;

-- select_sample_system_10.sql
SELECT *
FROM foo TABLESAMPLE SYSTEM (10)
;

-- select_sample_system_10_aliased.sql
SELECT f.colname
FROM foo AS f TABLESAMPLE SYSTEM (10)
;

-- select_sample_system_10_repeatable.sql
SELECT *
FROM foo TABLESAMPLE SYSTEM (10) REPEATABLE (100)
;

-- select_simple_a.sql
select 1
;

-- select_simple_b.sql
select * from blah
;

-- select_simple_c.sql
select * from foo, bar
;

-- select_simple_d.sql
 select 12 -- ends with comment
;

-- select_simple_e.sql
SELECT
    my_var::date as casted_variable,
    123::bigint as another_casted_number
FROM boo
;

-- select_simple_f.sql
-- test some more advanced constructs
SELECT
    a, b
FROM boo
GROUP BY 1
ORDER BY b, 1
LIMIT 5
;

-- select_simple_g.sql
-- Having Clause
SELECT
    id
FROM test
WHERE
    id >= 4
GROUP BY id
HAVING id < 5
;

-- select_simple_h.sql
-- test window functions in functions with casting
SELECT
    DATEADD(DAY, ROW_NUMBER() OVER (ORDER BY DateCD ASC), '2014-01-01') AS dt
FROM boo
;

-- select_simple_i.sql
-- test aliasing
SELECT
    raw_column,
    raw_sch.raw_col,
    simple_explicit as aliased_column_1,
    simple_implicit aliased_column_2,
    an_unaliased + calculation,
    -- We know that the following one doesn't parse...
    -- an_implicitly + aliased calculation,
    an_explicitly - aliased as calculation,
    'an unalised string',
    123.6,
    -786 as aliased_column3
FROM unaliased
JOIN aliased_1 as foo
JOIN aliased_2 bar USING(b)
;

-- select_simple_j.sql
-- test parsing of cross join and outer join
SELECT
    count_correctly_substituted
FROM
    correctly_substituted
CROSS JOIN
    needs_substitution
LEFT OUTER JOIN
    some_other_table
;

-- select_t.sql
SELECT
    *
FROM
    TABLE_1
    FULL OUTER JOIN  -- comment1
    (
        SELECT
            *
        FROM Table_B
        WHERE COL_2 = 'B'
        UNION ALL
        SELECT
            *
        FROM TABLE_C
        WHERE
            COL_1 = 0
    )
ON TABLE_1.A = TABLE_2.A
;

-- select_table_named_group.sql
select 1 from group;
select 1 from groups;

-- select_trailing_comma_column_list.sql
SELECT
    user_id,
FROM
    table
;

-- select_true_and_not_false.sql
-- https://github.com/sqlfluff/sqlfluff/issues/874
SELECT TRUE AND NOT FALSE;
SELECT TRUE;
SELECT TRUE AND FALSE;
SELECT NOT TRUE;
SELECT NOT TRUE AND FALSE;

-- select_u.sql
select
    substring(a from 'abc') as b
from my_table
;

-- select_union.sql
SELECT
  0
UNION
SELECT
  1
;

-- select_union_all.sql
SELECT
  0
UNION ALL
SELECT
  1
;

-- select_union_bracketed.sql
(SELECT 0) UNION (SELECT 1);

((SELECT 0)) UNION ((SELECT 1));

(
    SELECT * FROM tbl1
    EXCEPT
    SELECT * FROM tbl2
)
UNION
(
    SELECT * FROM tbl2
    EXCEPT
    SELECT * FROM tbl1
);

-- select_union_distinct.sql
SELECT
  0
UNION DISTINCT
SELECT
  1
;

-- select_v.sql
-- Test Nested WITH
WITH counter AS (
  WITH ladder AS (
    SELECT 1
  )
  SELECT *
  FROM ladder
  ORDER BY 1
)
SELECT *
FROM counter
;

-- select_where_in_unnest.sql
SELECT
    user_id
FROM
    t
WHERE
    1 IN UNNEST(t.c)
;

-- select_with_a.sql
WITH cte as (select a from tbla)
select a from cte
;

-- select_with_b.sql
WITH blah AS (select x,y,z FROM foo) select z, y, x from blah;

-- select_with_brackets.sql
select * from (my_table);

select * from (my_table tt);

select * from ((my_table tt));

select * from (((my_table tt)));

-- select_with_limit_and_offset.sql
SELECT *
FROM counter
LIMIT 10 OFFSET 10
;

-- select_with_offset_limit.sql
SELECT *
FROM counter
LIMIT 10, 10
;

-- select_with_recursive.sql
WITH RECURSIVE cte(a) AS (SELECT 1 UNION ALL SELECT a+1 FROM cte WHERE a < 5)
SELECT a FROM cte;

-- select_with_simple_limit.sql
SELECT *
FROM counter
LIMIT 10
;

-- select_with_where_clause_functions.sql
select
	t.column1
from
	sch.table1 as t
where
	t.b_year in (year(getdate()) , year(getdate()) - 1);

-- semicolon_delimiters.sql
-- It's possible to have multiple semicolons between statements.
SELECT foo FROM bar;;
SELECT foo FROM bar;
;
;
SELECT foo FROM bar;

-- set_order_by.sql
-- https://github.com/sqlfluff/sqlfluff/issues/852
SELECT 1 AS a
UNION ALL
SELECT 1 AS a
ORDER BY a
;

-- set_order_by_complex.sql
-- https://github.com/sqlfluff/sqlfluff/issues/852
-- ORDER BY and LIMIT are allowed when bracketed. Otherwise not.
(SELECT * FROM a ORDER BY 1 LIMIT 1)
UNION ALL
(SELECT * FROM b ORDER BY 1 LIMIT 1)
ORDER BY 1 LIMIT 1
;

-- set_schema_a.sql
set schema my_schema
;

-- shorthand_cast.sql
select
    '1'    ::   INT as id1,
    '2'::int as id2
from table_a
;

-- table_expression.sql
SELECT
    y AS woy
FROM
    UNNEST(GENERATE_ARRAY(1, 53)) AS y;

SELECT id, name
FROM
    UNNEST([1, 2, 3]) id WITH OFFSET pos1,
    UNNEST(['a', 'b', 'c']) name WITH OFFSET pos2
WHERE pos1 = pos2;

-- transactions.sql
BEGIN TRANSACTION;
UPDATE tbl SET foo = 1 WHERE bar = 2;
END TRANSACTION;

-- trim_functions.sql
SELECT trim('    SparkSQL   ');

SELECT trim(BOTH FROM '    SparkSQL   ');

SELECT trim(LEADING FROM '    SparkSQL   ');

SELECT trim(TRAILING FROM '    SparkSQL   ');

SELECT trim('SL' FROM 'SSparkSQLS');

SELECT trim(BOTH 'SL' FROM 'SSparkSQLS');

SELECT trim(LEADING 'SL' FROM 'SSparkSQLS');

SELECT trim(TRAILING 'SL' FROM 'SSparkSQLS');

-- truncate_a.sql
truncate a
;

-- truncate_table_a.sql
truncate table a
;

-- unaliased_using_subquery.sql
SELECT *
FROM A_TABLE
INNER JOIN (
    SELECT margin
    FROM B_TABLE
) USING (SOME_COLUMN)
;

-- update.sql
UPDATE table_name SET column1 = value1, column2 = value2 WHERE a=1;

-- update_set_case.sql
UPDATE table1
SET a = CASE WHEN t2.column = 'T' THEN TRUE
             WHEN t2.column = 'F' THEN FALSE
             ELSE NULL
        END
FROM table2 t2;

-- update_with_from_clause.sql
UPDATE
  my_table
SET
  my_table.days=other_table.days
FROM
  other_table
WHERE
  my_table.po_number=other_table.po_number
;

-- update_with_table_alias.sql
UPDATE
  my_table AS tttd
SET
  tttd.days=ttu.days
;

-- where_fetch.sql
SELECT * FROM Persons
WHERE Country='France'
FETCH FIRST 5 ROWS ONLY;

-- with_compound_select_union.sql
SELECT 0
UNION
(WITH t AS (SELECT 1) SELECT * FROM t)
;

(WITH t AS (SELECT 0) SELECT * FROM t)
UNION
SELECT 1
;

-- with_nested_in_with_statement.sql
(
    WITH mycte2 AS (
        WITH mycte1 AS (
            SELECT
                foo,
                bar,
                baz
            FROM mytable
        )
        SELECT
            foo,
            bar,
            baz
        FROM mycte1
    )
    SELECT
        foo,
        bar,
        baz
    FROM mycte2
);

-- with_no_schema_binding.sql
create view my_schema.my_view as
select * from schema.table
with no schema binding;
