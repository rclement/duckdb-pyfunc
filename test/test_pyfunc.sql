-- Test script for duckdb-pyfunc extension
-- Run with: duckdb -init test_pyfunc.sql

-- Load the extension
LOAD 'build/libpyfunc.so';

-- Test 1: Create a simple addition function
CREATE FUNCTION add_one(x INTEGER) RETURNS INTEGER LANGUAGE PYTHON AS $
return x + 1
$;

SELECT add_one(5);
-- Expected: 6

-- Test 2: Create a string function
CREATE FUNCTION greet(name VARCHAR) RETURNS VARCHAR LANGUAGE PYTHON AS $
return f"Hello, {name}!"
$;

SELECT greet('World');
-- Expected: Hello, World!

-- Test 3: Create a math function
CREATE FUNCTION square(x DOUBLE) RETURNS DOUBLE LANGUAGE PYTHON AS $
return x * x
$;

SELECT square(3.5);
-- Expected: 12.25

-- Test 4: Function with multiple parameters
CREATE FUNCTION add_numbers(a INTEGER, b INTEGER) RETURNS INTEGER LANGUAGE PYTHON AS $
return a + b
$;

SELECT add_numbers(10, 20);
-- Expected: 30

-- Test 5: List all registered functions
SELECT * FROM list_py_functions();

-- Test 6: Use Python imports
CREATE FUNCTION get_pi() RETURNS DOUBLE LANGUAGE PYTHON AS $
import math
return math.pi
$;

SELECT get_pi();
-- Expected: 3.14159...

-- Test 7: Drop a function
SELECT drop_py_function('add_one');

-- Verify it's dropped
SELECT * FROM list_py_functions();
