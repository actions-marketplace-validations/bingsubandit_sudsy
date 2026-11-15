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

