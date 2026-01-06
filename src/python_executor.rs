//! Python Executor using PyO3
//!
//! Handles executing Python code and converting values between Rust/DuckDB and Python types.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::parser::FunctionDefinition;

static PYTHON_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Python executor for running UDF code
pub struct PythonExecutor;

impl PythonExecutor {
    /// Initialize the Python interpreter (call once at extension load)
    pub fn initialize() -> Result<(), Box<dyn std::error::Error>> {
        if PYTHON_INITIALIZED.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already initialized
        }

        // PyO3's auto-initialize feature handles this, but we ensure it's ready
        Python::with_gil(|py| {
            // Import commonly used modules to warm up the interpreter
            py.import("builtins")?;
            Ok::<(), PyErr>(())
        })?;

        Ok(())
    }

    /// Create Python code that wraps the user's function body
    pub fn create_wrapper_code(func_def: &FunctionDefinition) -> String {
        let param_names: Vec<&str> = func_def.parameters.iter()
            .map(|p| p.name.as_str())
            .collect();
        let params_str = param_names.join(", ");

        // Indent the user's code properly
        let indented_body: String = func_def.body
            .lines()
            .map(|line| format!("    {}", line))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"def __pyfunc_wrapper__({params}):
{body}

__pyfunc_result__ = __pyfunc_wrapper__({params})
"#,
            params = params_str,
            body = indented_body
        )
    }

    /// Execute a Python UDF with the given arguments
    pub fn execute(
        func_def: &FunctionDefinition,
        args: &[DuckDBValue],
    ) -> Result<DuckDBValue, Box<dyn std::error::Error>> {
        Python::with_gil(|py| {
            // Create the execution namespace
            let globals = PyDict::new(py);
            let locals = PyDict::new(py);

            // Set up parameter values in locals
            for (param, arg) in func_def.parameters.iter().zip(args.iter()) {
                let py_value = arg.to_python(py)?;
                locals.set_item(&param.name, py_value)?;
            }

            // Create and execute the wrapper code
            let code = Self::create_wrapper_code(func_def);
            let code_cstr = CString::new(code)?;

            py.run(code_cstr.as_c_str(), Some(&globals), Some(&locals))?;

            // Get the result
            let result = locals.get_item("__pyfunc_result__")?
                .ok_or("Failed to get function result")?;

            // Convert back to DuckDB value
            DuckDBValue::from_python(result, &func_def.return_type)
        })
    }

    /// Execute a simple Python expression (for testing)
    #[allow(dead_code)]
    pub fn eval_expression(expr: &str) -> Result<DuckDBValue, Box<dyn std::error::Error>> {
        Python::with_gil(|py| {
            let expr_cstr = CString::new(expr)?;
            let result = py.eval(expr_cstr.as_c_str(), None, None)?;
            DuckDBValue::from_python_any(result)
        })
    }
}

/// Represents a value that can be passed between DuckDB and Python
#[derive(Debug, Clone)]
pub enum DuckDBValue {
    Null,
    Boolean(bool),
    TinyInt(i8),
    SmallInt(i16),
    Integer(i32),
    BigInt(i64),
    Float(f32),
    Double(f64),
    Varchar(String),
    Blob(Vec<u8>),
    // TODO: Add Date, Timestamp, etc.
}

impl DuckDBValue {
    /// Convert a DuckDB value to a Python object
    pub fn to_python<'py>(&self, py: Python<'py>) -> Result<Bound<'py, PyAny>, PyErr> {
        match self {
            DuckDBValue::Null => Ok(py.None().into_bound(py)),
            DuckDBValue::Boolean(b) => {
                let obj = (*b).into_pyobject(py)?;
                Ok(obj.to_owned().into_any())
            }
            DuckDBValue::TinyInt(i) => {
                let obj = (*i).into_pyobject(py)?;
                Ok(obj.into_any())
            }
            DuckDBValue::SmallInt(i) => {
                let obj = (*i).into_pyobject(py)?;
                Ok(obj.into_any())
            }
            DuckDBValue::Integer(i) => {
                let obj = (*i).into_pyobject(py)?;
                Ok(obj.into_any())
            }
            DuckDBValue::BigInt(i) => {
                let obj = (*i).into_pyobject(py)?;
                Ok(obj.into_any())
            }
            DuckDBValue::Float(f) => {
                let obj = (*f).into_pyobject(py)?;
                Ok(obj.into_any())
            }
            DuckDBValue::Double(f) => {
                let obj = (*f).into_pyobject(py)?;
                Ok(obj.into_any())
            }
            DuckDBValue::Varchar(s) => {
                let obj = s.as_str().into_pyobject(py)?;
                Ok(obj.into_any())
            }
            DuckDBValue::Blob(b) => {
                let obj = b.as_slice().into_pyobject(py)?;
                Ok(obj.into_any())
            }
        }
    }

    /// Convert a Python object to a DuckDB value with expected type
    pub fn from_python(obj: Bound<'_, PyAny>, expected_type: &str) -> Result<Self, Box<dyn std::error::Error>> {
        if obj.is_none() {
            return Ok(DuckDBValue::Null);
        }

        let result = match expected_type.to_uppercase().as_str() {
            "BOOLEAN" => {
                let val: bool = obj.extract()?;
                DuckDBValue::Boolean(val)
            }
            "TINYINT" => {
                let val: i8 = obj.extract()?;
                DuckDBValue::TinyInt(val)
            }
            "SMALLINT" => {
                let val: i16 = obj.extract()?;
                DuckDBValue::SmallInt(val)
            }
            "INTEGER" | "INT" => {
                let val: i32 = obj.extract()?;
                DuckDBValue::Integer(val)
            }
            "BIGINT" => {
                let val: i64 = obj.extract()?;
                DuckDBValue::BigInt(val)
            }
            "FLOAT" | "REAL" => {
                let val: f32 = obj.extract()?;
                DuckDBValue::Float(val)
            }
            "DOUBLE" => {
                let val: f64 = obj.extract()?;
                DuckDBValue::Double(val)
            }
            "VARCHAR" | "STRING" | "TEXT" => {
                let val: String = obj.extract()?;
                DuckDBValue::Varchar(val)
            }
            "BLOB" | "BINARY" => {
                let val: Vec<u8> = obj.extract()?;
                DuckDBValue::Blob(val)
            }
            _ => {
                // Try to infer from Python type
                Self::from_python_any(obj)?
            }
        };

        Ok(result)
    }

    /// Convert a Python object to a DuckDB value, inferring the type
    pub fn from_python_any(obj: Bound<'_, PyAny>) -> Result<Self, Box<dyn std::error::Error>> {
        if obj.is_none() {
            return Ok(DuckDBValue::Null);
        }

        // Check type and convert accordingly
        if let Ok(val) = obj.extract::<bool>() {
            // Note: must check bool before int since bool is a subclass of int in Python
            if obj.get_type().name()? == "bool" {
                return Ok(DuckDBValue::Boolean(val));
            }
        }

        if let Ok(val) = obj.extract::<i64>() {
            if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
                return Ok(DuckDBValue::Integer(val as i32));
            }
            return Ok(DuckDBValue::BigInt(val));
        }

        if let Ok(val) = obj.extract::<f64>() {
            return Ok(DuckDBValue::Double(val));
        }

        if let Ok(val) = obj.extract::<String>() {
            return Ok(DuckDBValue::Varchar(val));
        }

        if let Ok(val) = obj.extract::<Vec<u8>>() {
            return Ok(DuckDBValue::Blob(val));
        }

        // Fallback: convert to string
        let string_repr = obj.str()?.to_string();
        Ok(DuckDBValue::Varchar(string_repr))
    }

    /// Get the DuckDB type name for this value
    pub fn type_name(&self) -> &'static str {
        match self {
            DuckDBValue::Null => "NULL",
            DuckDBValue::Boolean(_) => "BOOLEAN",
            DuckDBValue::TinyInt(_) => "TINYINT",
            DuckDBValue::SmallInt(_) => "SMALLINT",
            DuckDBValue::Integer(_) => "INTEGER",
            DuckDBValue::BigInt(_) => "BIGINT",
            DuckDBValue::Float(_) => "FLOAT",
            DuckDBValue::Double(_) => "DOUBLE",
            DuckDBValue::Varchar(_) => "VARCHAR",
            DuckDBValue::Blob(_) => "BLOB",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::FunctionParameter;

    #[test]
    fn test_create_wrapper_code() {
        let func_def = FunctionDefinition {
            name: "greet".to_string(),
            parameters: vec![
                FunctionParameter {
                    name: "name".to_string(),
                    data_type: "VARCHAR".to_string(),
                },
            ],
            return_type: "VARCHAR".to_string(),
            language: "PYTHON".to_string(),
            body: "return 'Hello ' + name".to_string(),
        };

        let code = PythonExecutor::create_wrapper_code(&func_def);
        assert!(code.contains("def __pyfunc_wrapper__(name)"));
        assert!(code.contains("Hello"));
    }
}
