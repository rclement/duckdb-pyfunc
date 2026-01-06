//! Table function for creating Python UDFs
//!
//! Usage:
//!   SELECT * FROM create_py_function(
//!     'function_name',
//!     'param1 TYPE1, param2 TYPE2',
//!     'RETURN_TYPE',
//!     'python code body'
//!   );

use duckdb::{
    core::{DataChunkHandle, Inserter, LogicalTypeHandle, LogicalTypeId},
    vtab::{BindInfo, InitInfo, TableFunctionInfo, VTab},
    Result,
};
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::function_registry::{register_function, get_function, FUNCTION_REGISTRY};
use crate::parser::{FunctionDefinition, FunctionParameter, normalize_sql_type};
use crate::python_executor::{DuckDBValue, PythonExecutor};
use crate::scalar_function::string_to_logical_type;

/// Bind data for create_py_function
#[repr(C)]
pub struct CreateFunctionBindData {
    func_name: String,
    params_str: String,
    return_type: String,
    body: String,
    message: String,
}

/// Init data for create_py_function
#[repr(C)]
pub struct CreateFunctionInitData {
    done: AtomicBool,
}

/// Table function for creating Python UDFs
pub struct CreateFunctionVTab;

impl VTab for CreateFunctionVTab {
    type InitData = CreateFunctionInitData;
    type BindData = CreateFunctionBindData;

    fn bind(bind: &BindInfo) -> Result<Self::BindData, Box<dyn std::error::Error>> {
        // Output column shows result message
        bind.add_result_column("result", LogicalTypeHandle::from(LogicalTypeId::Varchar));

        // Get parameters
        let func_name = bind.get_parameter(0).to_string();
        let params_str = bind.get_parameter(1).to_string();
        let return_type = bind.get_parameter(2).to_string();
        let body = bind.get_parameter(3).to_string();

        // Parse parameters
        let parameters = parse_params_string(&params_str)?;

        // Create function definition
        let func_def = FunctionDefinition {
            name: func_name.clone(),
            parameters,
            return_type: normalize_sql_type(&return_type),
            language: "PYTHON".to_string(),
            body: body.clone(),
        };

        // Register in the function registry
        register_function(func_def)?;

        let message = format!("Function '{}' created successfully", func_name);

        Ok(CreateFunctionBindData {
            func_name,
            params_str,
            return_type,
            body,
            message,
        })
    }

    fn init(_: &InitInfo) -> Result<Self::InitData, Box<dyn std::error::Error>> {
        Ok(CreateFunctionInitData {
            done: AtomicBool::new(false),
        })
    }

    fn func(
        func: &TableFunctionInfo<Self>,
        output: &mut DataChunkHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let init_data = func.get_init_data();
        let bind_data = func.get_bind_data();

        if init_data.done.swap(true, Ordering::Relaxed) {
            output.set_len(0);
        } else {
            let vector = output.flat_vector(0);
            let result = CString::new(bind_data.message.clone())?;
            vector.insert(0, result);
            output.set_len(1);
        }
        Ok(())
    }

    fn parameters() -> Option<Vec<LogicalTypeHandle>> {
        Some(vec![
            LogicalTypeHandle::from(LogicalTypeId::Varchar), // function name
            LogicalTypeHandle::from(LogicalTypeId::Varchar), // parameters
            LogicalTypeHandle::from(LogicalTypeId::Varchar), // return type
            LogicalTypeHandle::from(LogicalTypeId::Varchar), // body
        ])
    }
}

/// Parse a parameter string like "name1 TYPE1, name2 TYPE2"
fn parse_params_string(params_str: &str) -> Result<Vec<FunctionParameter>, Box<dyn std::error::Error>> {
    let trimmed = params_str.trim();
    if trimmed.is_empty() {
        return Ok(vec![]);
    }

    let mut parameters = Vec::new();

    for param in trimmed.split(',') {
        let param = param.trim();
        if param.is_empty() {
            continue;
        }

        let parts: Vec<&str> = param.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(format!("Invalid parameter definition: '{}'. Expected 'name TYPE'", param).into());
        }

        let name = parts[0].to_string();
        let data_type = normalize_sql_type(&parts[1..].join(" "));

        parameters.push(FunctionParameter { name, data_type });
    }

    Ok(parameters)
}

// ============================================================================
// py_call table function - for calling registered Python functions
// ============================================================================

/// Bind data for py_call
#[repr(C)]
pub struct PyCallBindData {
    func_name: String,
    args: Vec<String>,
}

/// Init data for py_call
#[repr(C)]
pub struct PyCallInitData {
    done: AtomicBool,
}

/// Table function for calling Python UDFs
pub struct PyCallVTab;

impl VTab for PyCallVTab {
    type InitData = PyCallInitData;
    type BindData = PyCallBindData;

    fn bind(bind: &BindInfo) -> Result<Self::BindData, Box<dyn std::error::Error>> {
        // Get function name from first parameter
        let func_name = bind.get_parameter(0).to_string();

        // Look up the function to get return type
        let func_def = get_function(&func_name)
            .ok_or_else(|| format!("Function '{}' not found", func_name))?;

        // Set return type based on function definition
        let return_type = string_to_logical_type(&func_def.return_type);
        bind.add_result_column("result", LogicalTypeHandle::from(return_type));

        // Collect additional arguments
        let mut args = Vec::new();
        let param_count = bind.get_parameter_count();
        for i in 1..param_count {
            args.push(bind.get_parameter(i).to_string());
        }

        Ok(PyCallBindData { func_name, args })
    }

    fn init(_: &InitInfo) -> Result<Self::InitData, Box<dyn std::error::Error>> {
        Ok(PyCallInitData {
            done: AtomicBool::new(false),
        })
    }

    fn func(
        func: &TableFunctionInfo<Self>,
        output: &mut DataChunkHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let init_data = func.get_init_data();
        let bind_data = func.get_bind_data();

        if init_data.done.swap(true, Ordering::Relaxed) {
            output.set_len(0);
            return Ok(());
        }

        // Get function definition
        let func_def = get_function(&bind_data.func_name)
            .ok_or_else(|| format!("Function '{}' not found", bind_data.func_name))?;

        // Convert string args to DuckDBValue based on expected types
        let mut args: Vec<DuckDBValue> = Vec::new();
        for (i, arg_str) in bind_data.args.iter().enumerate() {
            if i < func_def.parameters.len() {
                let param_type = &func_def.parameters[i].data_type;
                let value = parse_string_to_value(arg_str, param_type)?;
                args.push(value);
            } else {
                // Extra argument, treat as varchar
                args.push(DuckDBValue::Varchar(arg_str.clone()));
            }
        }

        // Execute Python function
        let result = PythonExecutor::execute(&func_def, &args)?;

        // Set output
        let vector = output.flat_vector(0);
        match result {
            DuckDBValue::Null => {
                // Set null - this is tricky with the current API
                output.set_len(0);
                return Ok(());
            }
            DuckDBValue::Varchar(s) => {
                let c_str = CString::new(s)?;
                vector.insert(0, c_str);
            }
            DuckDBValue::Integer(i) => {
                // For non-varchar types, we need to handle differently
                // The insert method expects CString for varchar
                // This is a limitation of the current approach
                let c_str = CString::new(i.to_string())?;
                vector.insert(0, c_str);
            }
            DuckDBValue::BigInt(i) => {
                let c_str = CString::new(i.to_string())?;
                vector.insert(0, c_str);
            }
            DuckDBValue::Double(f) => {
                let c_str = CString::new(f.to_string())?;
                vector.insert(0, c_str);
            }
            DuckDBValue::Boolean(b) => {
                let c_str = CString::new(if b { "true" } else { "false" })?;
                vector.insert(0, c_str);
            }
            _ => {
                let c_str = CString::new("Unsupported return type")?;
                vector.insert(0, c_str);
            }
        }

        output.set_len(1);
        Ok(())
    }

    fn parameters() -> Option<Vec<LogicalTypeHandle>> {
        // Variable number of parameters - function name + args
        Some(vec![
            LogicalTypeHandle::from(LogicalTypeId::Varchar), // function name
        ])
    }
}

/// Parse a string value to DuckDBValue based on expected type
fn parse_string_to_value(s: &str, type_name: &str) -> Result<DuckDBValue, Box<dyn std::error::Error>> {
    let upper_type = type_name.to_uppercase();

    match upper_type.as_str() {
        "BOOLEAN" | "BOOL" => {
            let lower = s.to_lowercase();
            let val = lower == "true" || lower == "1" || lower == "yes";
            Ok(DuckDBValue::Boolean(val))
        }
        "TINYINT" | "INT8" => {
            let val: i8 = s.parse()?;
            Ok(DuckDBValue::TinyInt(val))
        }
        "SMALLINT" | "INT16" => {
            let val: i16 = s.parse()?;
            Ok(DuckDBValue::SmallInt(val))
        }
        "INTEGER" | "INT" | "INT32" => {
            let val: i32 = s.parse()?;
            Ok(DuckDBValue::Integer(val))
        }
        "BIGINT" | "INT64" => {
            let val: i64 = s.parse()?;
            Ok(DuckDBValue::BigInt(val))
        }
        "FLOAT" | "REAL" => {
            let val: f32 = s.parse()?;
            Ok(DuckDBValue::Float(val))
        }
        "DOUBLE" => {
            let val: f64 = s.parse()?;
            Ok(DuckDBValue::Double(val))
        }
        "VARCHAR" | "STRING" | "TEXT" => {
            Ok(DuckDBValue::Varchar(s.to_string()))
        }
        _ => {
            Ok(DuckDBValue::Varchar(s.to_string()))
        }
    }
}

// ============================================================================
// list_py_functions table function - lists all registered functions
// ============================================================================

/// Bind data for list_py_functions
#[repr(C)]
pub struct ListFunctionsBindData {
    functions: Vec<(String, String, String)>, // (name, params, return_type)
}

/// Init data for list_py_functions
#[repr(C)]
pub struct ListFunctionsInitData {
    current_index: std::sync::atomic::AtomicUsize,
}

/// Table function for listing Python UDFs
pub struct ListFunctionsVTab;

impl VTab for ListFunctionsVTab {
    type InitData = ListFunctionsInitData;
    type BindData = ListFunctionsBindData;

    fn bind(bind: &BindInfo) -> Result<Self::BindData, Box<dyn std::error::Error>> {
        bind.add_result_column("name", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("parameters", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        bind.add_result_column("return_type", LogicalTypeHandle::from(LogicalTypeId::Varchar));

        // Get all functions from registry
        let registry = FUNCTION_REGISTRY.lock();
        let functions: Vec<(String, String, String)> = registry
            .iter()
            .map(|(name, def)| {
                let params_str = def.parameters
                    .iter()
                    .map(|p| format!("{} {}", p.name, p.data_type))
                    .collect::<Vec<_>>()
                    .join(", ");
                (name.clone(), params_str, def.return_type.clone())
            })
            .collect();

        Ok(ListFunctionsBindData { functions })
    }

    fn init(_: &InitInfo) -> Result<Self::InitData, Box<dyn std::error::Error>> {
        Ok(ListFunctionsInitData {
            current_index: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    fn func(
        func: &TableFunctionInfo<Self>,
        output: &mut DataChunkHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let init_data = func.get_init_data();
        let bind_data = func.get_bind_data();

        let idx = init_data.current_index.fetch_add(1, Ordering::Relaxed);

        if idx >= bind_data.functions.len() {
            output.set_len(0);
            return Ok(());
        }

        let (name, params, return_type) = &bind_data.functions[idx];

        let name_vec = output.flat_vector(0);
        let params_vec = output.flat_vector(1);
        let return_vec = output.flat_vector(2);

        name_vec.insert(0, CString::new(name.as_str())?);
        params_vec.insert(0, CString::new(params.as_str())?);
        return_vec.insert(0, CString::new(return_type.as_str())?);

        output.set_len(1);
        Ok(())
    }

    fn parameters() -> Option<Vec<LogicalTypeHandle>> {
        Some(vec![]) // No parameters
    }
}
