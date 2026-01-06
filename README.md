# DuckDB PyFunc Extension

A DuckDB extension that allows registering Python User Defined Functions (UDFs) directly from SQL statements, without requiring the DuckDB Python package.

## Features

- Define Python UDFs using SQL syntax similar to Databricks Spark
- Execute Python code within DuckDB queries
- Support for various data types (VARCHAR, INTEGER, DOUBLE, BOOLEAN, etc.)
- No Python SDK required - just load the extension and use SQL

## Installation

### Prerequisites

- Rust toolchain (1.70+)
- Python 3.8+ with development headers
- DuckDB 1.4.3+

### Building

```bash
# Clone the repository
git clone <repo-url>
cd duckdb-pyfunc

# Build debug version
make debug

# Build release version
make release
```

## Usage

### Loading the Extension

```sql
-- Start DuckDB with unsigned extension support
-- duckdb -unsigned

LOAD './path/to/duckdb_pyfunc.duckdb_extension';
```

### Creating a Python UDF

Use the `create_py_function` table function to define a new Python UDF:

```sql
-- Syntax: create_py_function(name, parameters, return_type, body)
SELECT * FROM create_py_function(
    'greet',
    'name VARCHAR',
    'VARCHAR',
    'return "Hello " + name + "!"'
);
```

### Calling a Python UDF

Use the `py_call` table function to invoke registered UDFs:

```sql
SELECT * FROM py_call('greet', 'World');
-- Returns: Hello World!
```

### Listing Registered Functions

```sql
SELECT * FROM list_py_functions();
```

## Examples

### String Functions

```sql
-- Create a function to reverse a string
SELECT * FROM create_py_function(
    'reverse_str',
    's VARCHAR',
    'VARCHAR',
    'return s[::-1]'
);

SELECT * FROM py_call('reverse_str', 'hello');
-- Returns: olleh
```

### Numeric Functions

```sql
-- Create a function to calculate factorial
SELECT * FROM create_py_function(
    'factorial',
    'n INTEGER',
    'INTEGER',
    'result = 1
for i in range(1, n + 1):
    result *= i
return result'
);

SELECT * FROM py_call('factorial', '5');
-- Returns: 120
```

### Using Python Libraries

```sql
-- Use the math library
SELECT * FROM create_py_function(
    'circle_area',
    'radius DOUBLE',
    'DOUBLE',
    'import math
return math.pi * radius ** 2'
);

SELECT * FROM py_call('circle_area', '5.0');
-- Returns: 78.53981633974483
```

### Multiple Parameters

```sql
-- Function with multiple parameters
SELECT * FROM create_py_function(
    'full_name',
    'first VARCHAR, last VARCHAR',
    'VARCHAR',
    'return first + " " + last'
);

SELECT * FROM py_call('full_name', 'John', 'Doe');
-- Returns: John Doe
```

## SQL Syntax Reference

The extension supports Databricks-style CREATE FUNCTION syntax for reference:

```sql
CREATE FUNCTION function_name(param1 TYPE1, param2 TYPE2, ...)
RETURNS return_type
LANGUAGE PYTHON
AS $$
python_code_here
$$
```

Currently implemented via table functions:
- `create_py_function(name, params, return_type, body)` - Register a UDF
- `py_call(name, arg1, arg2, ...)` - Call a registered UDF
- `list_py_functions()` - List all registered UDFs

## Supported Data Types

| SQL Type | Python Type |
|----------|-------------|
| VARCHAR, STRING, TEXT | str |
| INTEGER, INT | int |
| BIGINT | int |
| SMALLINT, TINYINT | int |
| FLOAT, REAL | float |
| DOUBLE | float |
| BOOLEAN, BOOL | bool |
| BLOB, BINARY | bytes |

## Development

### Running Tests

```bash
make test
```

### Code Quality

```bash
make check    # Run cargo check
make clippy   # Run clippy linter
make fmt      # Format code
```

### Running DuckDB with Extension

```bash
make run
```

## Architecture

The extension is built with:
- **Rust** - Core extension logic
- **PyO3** - Python/Rust interoperability
- **DuckDB C API** - Extension integration

### Module Structure

- `lib.rs` - Extension entry point
- `parser.rs` - SQL syntax parser
- `python_executor.rs` - PyO3 Python execution
- `function_registry.rs` - UDF storage
- `scalar_function.rs` - DuckDB function registration
- `create_function_vtab.rs` - Table function implementations

## License

MIT License

## Acknowledgments

- [DuckDB](https://duckdb.org/) - The database engine
- [PyO3](https://pyo3.rs/) - Rust bindings for Python
- [Databricks](https://databricks.com/) - SQL syntax inspiration
