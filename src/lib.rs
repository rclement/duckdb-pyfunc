extern crate duckdb;
extern crate duckdb_loadable_macros;
extern crate libduckdb_sys;

mod parser;
mod python_executor;
mod function_registry;
mod scalar_function;
mod create_function_vtab;

use duckdb::{Connection, Result};
use duckdb_loadable_macros::duckdb_entrypoint_c_api;
use libduckdb_sys as ffi;
use std::error::Error;

use crate::create_function_vtab::{CreateFunctionVTab, PyCallVTab, ListFunctionsVTab};
use crate::python_executor::PythonExecutor;

const EXTENSION_NAME: &str = "pyfunc";

/// Initialize the Python interpreter when the extension loads
fn init_python() -> std::result::Result<(), Box<dyn Error>> {
    PythonExecutor::initialize()?;
    Ok(())
}

#[duckdb_entrypoint_c_api()]
pub unsafe fn extension_entrypoint(con: Connection) -> Result<(), Box<dyn Error>> {
    // Initialize Python
    init_python()?;

    // Register the create_py_function table function for defining new Python UDFs
    // Usage: SELECT * FROM create_py_function('name', 'params', 'return_type', 'body')
    con.register_table_function::<CreateFunctionVTab>("create_py_function")
        .expect("Failed to register create_py_function");

    // Register the py_call table function for invoking Python UDFs
    // Usage: SELECT * FROM py_call('func_name', arg1, arg2, ...)
    con.register_table_function::<PyCallVTab>("py_call")
        .expect("Failed to register py_call");

    // Register the list_py_functions table function
    // Usage: SELECT * FROM list_py_functions()
    con.register_table_function::<ListFunctionsVTab>("list_py_functions")
        .expect("Failed to register list_py_functions");

    Ok(())
}
