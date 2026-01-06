# DuckDB PyFunc Extension

A DuckDB extension that allows registering Python User Defined Functions (UDFs) directly from SQL statements using native Databricks Spark-style syntax.

## Features

- **Native SQL Syntax**: Use `CREATE FUNCTION ... LANGUAGE PYTHON AS $ ... $` syntax directly in SQL
- Execute Python code within DuckDB queries
- Support for various data types (VARCHAR, INTEGER, DOUBLE, BOOLEAN, DATE, TIMESTAMP, etc.)
- Embedded Python interpreter using pybind11
- No Python SDK required - just load the extension and use SQL

## Installation

### Prerequisites

- CMake 3.15+
- C++17 compatible compiler
- Python 3.8+ with development headers
- DuckDB 1.1.3+ (automatically fetched if not found)
- pybind11 (automatically fetched if not found)

### Building

```bash
# Clone the repository
git clone <repo-url>
cd duckdb-pyfunc

# Build using the build script
./scripts/build.sh

# Or manually with CMake
mkdir build && cd build
cmake -DCMAKE_BUILD_TYPE=Release ..
cmake --build . -j$(nproc)
```

## Usage

### Loading the Extension

```sql
-- Start DuckDB with unsigned extension support
-- duckdb -unsigned

LOAD 'build/libpyfunc.so';  -- or .dylib on macOS
```

### Creating a Python UDF

Use the native Databricks-style `CREATE FUNCTION` syntax:

```sql
-- Simple function
CREATE FUNCTION greet(name VARCHAR) RETURNS VARCHAR LANGUAGE PYTHON AS $
return f"Hello, {name}!"
$;

-- Function with multiple parameters
CREATE FUNCTION add_numbers(a INTEGER, b INTEGER) RETURNS INTEGER LANGUAGE PYTHON AS $
return a + b
$;

-- Function using Python libraries
CREATE FUNCTION circle_area(radius DOUBLE) RETURNS DOUBLE LANGUAGE PYTHON AS $
import math
return math.pi * radius ** 2
$;
```

### Calling Python UDFs

Once defined, use the function like any native DuckDB function:

```sql
SELECT greet('World');
-- Returns: Hello, World!

SELECT add_numbers(10, 20);
-- Returns: 30

SELECT circle_area(5.0);
-- Returns: 78.53981633974483
```

### Listing Registered Functions

```sql
SELECT * FROM list_py_functions();
```

### Dropping Functions

```sql
SELECT drop_py_function('function_name');
```

## Examples

### String Functions

```sql
CREATE FUNCTION reverse_str(s VARCHAR) RETURNS VARCHAR LANGUAGE PYTHON AS $
return s[::-1]
$;

SELECT reverse_str('hello');
-- Returns: olleh
```

### Numeric Functions

```sql
CREATE FUNCTION factorial(n INTEGER) RETURNS INTEGER LANGUAGE PYTHON AS $
result = 1
for i in range(1, n + 1):
    result *= i
return result
$;

SELECT factorial(5);
-- Returns: 120
```

### Date Functions

```sql
CREATE FUNCTION days_until_christmas(d DATE) RETURNS INTEGER LANGUAGE PYTHON AS $
import datetime
christmas = datetime.date(d.year, 12, 25)
if d > christmas:
    christmas = datetime.date(d.year + 1, 12, 25)
return (christmas - d).days
$;

SELECT days_until_christmas(DATE '2024-12-01');
-- Returns: 24
```

### Using External Libraries

```sql
-- Requires numpy to be installed in the Python environment
CREATE FUNCTION numpy_mean(values VARCHAR) RETURNS DOUBLE LANGUAGE PYTHON AS $
import numpy as np
import json
arr = json.loads(values)
return float(np.mean(arr))
$;

SELECT numpy_mean('[1, 2, 3, 4, 5]');
-- Returns: 3.0
```

## SQL Syntax Reference

```sql
CREATE FUNCTION function_name(param1 TYPE1, param2 TYPE2, ...)
RETURNS return_type
LANGUAGE PYTHON
AS $
python_code_here
$;
```

### Supported Parameter Types

| SQL Type | Python Type |
|----------|-------------|
| VARCHAR, STRING, TEXT | str |
| INTEGER, INT | int |
| BIGINT | int |
| SMALLINT | int |
| TINYINT | int |
| FLOAT, REAL | float |
| DOUBLE | float |
| BOOLEAN, BOOL | bool |
| BLOB, BINARY | bytes |
| DATE | datetime.date |
| TIMESTAMP | datetime.datetime |

## Development

### Running Tests

```bash
# Run the SQL test script
duckdb -unsigned < test/test_pyfunc.sql
```

### Project Structure

```
duckdb-pyfunc/
├── CMakeLists.txt              # Build configuration
├── src/
│   ├── include/
│   │   └── pyfunc_extension.hpp  # Main header
│   ├── pyfunc_extension.cpp      # Extension entry point
│   ├── pyfunc_parser.cpp         # SQL parser extension
│   ├── pyfunc_function.cpp       # Python executor
│   └── pyfunc_registry.cpp       # Function registry
├── scripts/
│   └── build.sh                  # Build script
└── test/
    └── test_pyfunc.sql           # SQL tests
```

## Architecture

The extension is built with:
- **C++17** - Core extension logic
- **pybind11** - Python/C++ interoperability (embedded interpreter)
- **DuckDB C++ API** - Parser extension and function registration

### Key Components

- **PyFuncParserExtension**: Parses `CREATE FUNCTION ... LANGUAGE PYTHON` statements
- **PyFunctionRegistry**: Stores registered Python function definitions
- **PyExecutor**: Manages the embedded Python interpreter and executes functions
- **Type Converters**: Convert between DuckDB Values and Python objects

## Limitations

- Functions are stored in memory and not persisted across sessions
- The Python interpreter is shared across all functions
- Global state in Python functions persists between calls

## License

MIT License

## Acknowledgments

- [DuckDB](https://duckdb.org/) - The database engine
- [pybind11](https://pybind11.readthedocs.io/) - C++/Python interoperability
- [Databricks](https://databricks.com/) - SQL syntax inspiration
