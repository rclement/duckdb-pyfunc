//! DuckDB Scalar Function Registration
//!
//! This module provides utilities for type conversion and helper functions.
//! The main UDF functionality is provided via table functions in create_function_vtab.rs

use duckdb::core::LogicalTypeId;

/// Convert a DuckDB type string to LogicalTypeId
pub fn string_to_logical_type(type_str: &str) -> LogicalTypeId {
    match type_str.to_uppercase().as_str() {
        "BOOLEAN" | "BOOL" => LogicalTypeId::Boolean,
        "TINYINT" | "INT8" => LogicalTypeId::Tinyint,
        "SMALLINT" | "INT16" => LogicalTypeId::Smallint,
        "INTEGER" | "INT" | "INT32" => LogicalTypeId::Integer,
        "BIGINT" | "INT64" => LogicalTypeId::Bigint,
        "FLOAT" | "FLOAT32" | "REAL" => LogicalTypeId::Float,
        "DOUBLE" | "FLOAT64" => LogicalTypeId::Double,
        "VARCHAR" | "STRING" | "TEXT" => LogicalTypeId::Varchar,
        "BLOB" | "BINARY" => LogicalTypeId::Blob,
        "DATE" => LogicalTypeId::Date,
        "TIMESTAMP" => LogicalTypeId::Timestamp,
        _ => LogicalTypeId::Varchar, // Default to varchar
    }
}

/// Convert LogicalTypeId to string representation
#[allow(dead_code)]
pub fn logical_type_to_string(type_id: LogicalTypeId) -> &'static str {
    match type_id {
        LogicalTypeId::Boolean => "BOOLEAN",
        LogicalTypeId::Tinyint => "TINYINT",
        LogicalTypeId::Smallint => "SMALLINT",
        LogicalTypeId::Integer => "INTEGER",
        LogicalTypeId::Bigint => "BIGINT",
        LogicalTypeId::Float => "FLOAT",
        LogicalTypeId::Double => "DOUBLE",
        LogicalTypeId::Varchar => "VARCHAR",
        LogicalTypeId::Blob => "BLOB",
        LogicalTypeId::Date => "DATE",
        LogicalTypeId::Timestamp => "TIMESTAMP",
        _ => "VARCHAR",
    }
}
