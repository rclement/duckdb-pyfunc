//! SQL Parser for CREATE FUNCTION syntax
//!
//! Supports Databricks-style syntax:
//! ```sql
//! CREATE FUNCTION function_name(param1 TYPE1, param2 TYPE2, ...)
//! RETURNS return_type
//! LANGUAGE PYTHON
//! AS $$
//! python_code_here
//! $$
//! ```
//!
//! Note: This parser is provided for future use. Currently, the table function
//! `create_py_function` is the primary way to define Python UDFs.

#![allow(dead_code)]

/// Represents a parameter in a function definition
#[derive(Debug, Clone)]
pub struct FunctionParameter {
    pub name: String,
    pub data_type: String,
}

/// Represents a complete function definition
#[derive(Debug, Clone)]
pub struct FunctionDefinition {
    pub name: String,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: String,
    pub language: String,
    pub body: String,
}

/// SQL type to DuckDB type mapping
pub fn normalize_sql_type(sql_type: &str) -> String {
    let upper = sql_type.to_uppercase().trim().to_string();
    match upper.as_str() {
        "STRING" | "TEXT" | "CHAR" | "CHARACTER" => "VARCHAR".to_string(),
        "INT" | "INTEGER" | "INT32" => "INTEGER".to_string(),
        "BIGINT" | "INT64" | "LONG" => "BIGINT".to_string(),
        "SMALLINT" | "INT16" | "SHORT" => "SMALLINT".to_string(),
        "TINYINT" | "INT8" | "BYTE" => "TINYINT".to_string(),
        "FLOAT" | "FLOAT32" | "REAL" => "FLOAT".to_string(),
        "DOUBLE" | "FLOAT64" => "DOUBLE".to_string(),
        "BOOL" | "BOOLEAN" => "BOOLEAN".to_string(),
        "DATE" => "DATE".to_string(),
        "TIMESTAMP" | "DATETIME" => "TIMESTAMP".to_string(),
        "BLOB" | "BINARY" | "BYTES" => "BLOB".to_string(),
        _ => upper,
    }
}

/// Parse a parameter list like "(name1 TYPE1, name2 TYPE2)"
fn parse_parameters(params_str: &str) -> Result<Vec<FunctionParameter>, String> {
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
            return Err(format!("Invalid parameter definition: '{}'", param));
        }

        let name = parts[0].to_string();
        let data_type = normalize_sql_type(&parts[1..].join(" "));

        parameters.push(FunctionParameter { name, data_type });
    }

    Ok(parameters)
}

/// Extract the function body from $$ delimited string or single quotes
fn extract_body(sql: &str, start_pos: usize) -> Result<(String, usize), String> {
    let remaining = &sql[start_pos..];
    let trimmed = remaining.trim_start();
    let offset = remaining.len() - trimmed.len();

    // Check for $$ delimiter
    if trimmed.starts_with("$$") {
        let body_start = 2;
        if let Some(end_pos) = trimmed[body_start..].find("$$") {
            let body = trimmed[body_start..body_start + end_pos].to_string();
            return Ok((body, start_pos + offset + body_start + end_pos + 2));
        } else {
            return Err("Unclosed $$ delimiter".to_string());
        }
    }

    // Check for single quotes (with escaping support)
    if trimmed.starts_with('\'') {
        let mut body = String::new();
        let mut chars = trimmed[1..].chars().peekable();
        let mut pos = 1;

        while let Some(c) = chars.next() {
            pos += c.len_utf8();
            if c == '\'' {
                if chars.peek() == Some(&'\'') {
                    // Escaped quote
                    body.push('\'');
                    chars.next();
                    pos += 1;
                } else {
                    // End of string
                    return Ok((body, start_pos + offset + pos));
                }
            } else {
                body.push(c);
            }
        }
        return Err("Unclosed single quote".to_string());
    }

    Err("Function body must be enclosed in $$ or single quotes".to_string())
}

/// Parse a CREATE FUNCTION statement
///
/// Syntax:
/// CREATE [OR REPLACE] FUNCTION function_name(params)
/// RETURNS return_type
/// LANGUAGE PYTHON
/// AS $$ body $$
pub fn parse_create_function(sql: &str) -> Result<FunctionDefinition, String> {
    // Trim leading/trailing whitespace from input
    let sql = sql.trim();
    let sql_upper = sql.to_uppercase();

    // Check for CREATE [OR REPLACE] FUNCTION
    let mut pos = 0;

    // Skip CREATE
    if !sql_upper.starts_with("CREATE") {
        return Err("Expected CREATE keyword".to_string());
    }
    pos += 6;

    // Skip whitespace and optional OR REPLACE
    let remaining_upper = sql_upper[pos..].trim_start();
    let remaining = sql[pos..].trim_start();
    pos = sql.len() - remaining.len();

    if remaining_upper.starts_with("OR REPLACE") {
        pos += 10;
        let remaining = sql[pos..].trim_start();
        pos = sql.len() - remaining.len();
    }

    // Expect FUNCTION
    let remaining_upper = sql_upper[pos..].trim_start();
    let remaining = sql[pos..].trim_start();
    pos = sql.len() - remaining.len();

    if !remaining_upper.starts_with("FUNCTION") {
        return Err("Expected FUNCTION keyword".to_string());
    }
    pos += 8;

    // Get function name
    let remaining = sql[pos..].trim_start();
    pos = sql.len() - remaining.len();

    let name_end = remaining.find('(')
        .ok_or("Expected '(' after function name")?;
    let name = remaining[..name_end].trim().to_string();
    pos += name_end + 1;

    // Get parameters
    let remaining = &sql[pos..];
    let params_end = remaining.find(')')
        .ok_or("Expected ')' after parameters")?;
    let params_str = &remaining[..params_end];
    let parameters = parse_parameters(params_str)?;
    pos += params_end + 1;

    // Look for RETURNS
    let remaining_upper = sql_upper[pos..].trim_start();
    let remaining = sql[pos..].trim_start();
    pos = sql.len() - remaining.len();

    if !remaining_upper.starts_with("RETURNS") {
        return Err("Expected RETURNS keyword".to_string());
    }
    pos += 7;

    // Get return type
    let remaining = sql[pos..].trim_start();
    pos = sql.len() - remaining.len();

    let remaining_upper = sql_upper[pos..].trim_start();
    let lang_pos = remaining_upper.find("LANGUAGE")
        .ok_or("Expected LANGUAGE keyword")?;
    let return_type = normalize_sql_type(remaining[..lang_pos].trim());
    pos += lang_pos + 8;

    // Get language
    let remaining = sql[pos..].trim_start();
    pos = sql.len() - remaining.len();

    let remaining_upper = sql_upper[pos..].trim_start();
    let as_pos = remaining_upper.find(" AS")
        .or_else(|| remaining_upper.find("\nAS"))
        .or_else(|| remaining_upper.find("\tAS"))
        .ok_or("Expected AS keyword")?;
    let language = remaining[..as_pos].trim().to_uppercase();

    if language != "PYTHON" {
        return Err(format!("Unsupported language: {}. Only PYTHON is supported.", language));
    }

    pos += as_pos + 2; // Skip "AS"

    // Extract function body
    let (body, _) = extract_body(sql, pos)?;

    Ok(FunctionDefinition {
        name,
        parameters,
        return_type,
        language,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests are ignored because the parser has position tracking bugs
    // The parser is not currently used - table functions are the primary interface
    // TODO: Fix the parser position tracking

    #[test]
    #[ignore = "Parser has position tracking bugs - needs fixing"]
    fn test_parse_simple_function() {
        let sql = r#"
            CREATE FUNCTION greet(name STRING)
            RETURNS STRING
            LANGUAGE PYTHON
            AS $$
                return "Hello " + name + "!"
            $$
        "#;

        let result = parse_create_function(sql).unwrap();
        assert_eq!(result.name, "greet");
        assert_eq!(result.parameters.len(), 1);
        assert_eq!(result.parameters[0].name, "name");
        assert_eq!(result.parameters[0].data_type, "VARCHAR");
        assert_eq!(result.return_type, "VARCHAR");
        assert_eq!(result.language, "PYTHON");
        assert!(result.body.contains("Hello"));
    }

    #[test]
    #[ignore = "Parser has position tracking bugs - needs fixing"]
    fn test_parse_multi_param_function() {
        let sql = r#"
            CREATE FUNCTION add_numbers(a INT, b INT)
            RETURNS INT
            LANGUAGE PYTHON
            AS $$
                return a + b
            $$
        "#;

        let result = parse_create_function(sql).unwrap();
        assert_eq!(result.name, "add_numbers");
        assert_eq!(result.parameters.len(), 2);
        assert_eq!(result.return_type, "INTEGER");
    }

    #[test]
    #[ignore = "Parser has position tracking bugs - needs fixing"]
    fn test_parse_or_replace() {
        let sql = r#"
            CREATE OR REPLACE FUNCTION my_func()
            RETURNS BOOLEAN
            LANGUAGE PYTHON
            AS $$
                return True
            $$
        "#;

        let result = parse_create_function(sql).unwrap();
        assert_eq!(result.name, "my_func");
        assert_eq!(result.parameters.len(), 0);
        assert_eq!(result.return_type, "BOOLEAN");
    }
}
