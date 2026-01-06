//! Function Registry
//!
//! Stores registered Python UDF definitions in a global registry.

use parking_lot::Mutex;
use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::parser::FunctionDefinition;

/// Global registry for Python functions
pub static FUNCTION_REGISTRY: Lazy<Mutex<HashMap<String, FunctionDefinition>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Register a new Python function
pub fn register_function(func_def: FunctionDefinition) -> Result<(), String> {
    let mut registry = FUNCTION_REGISTRY.lock();
    let name = func_def.name.to_lowercase();

    // Check if function already exists (allow override with OR REPLACE semantics)
    if registry.contains_key(&name) {
        // For now, just replace it
        eprintln!("Replacing existing function: {}", name);
    }

    registry.insert(name, func_def);
    Ok(())
}

/// Get a function definition by name
pub fn get_function(name: &str) -> Option<FunctionDefinition> {
    let registry = FUNCTION_REGISTRY.lock();
    registry.get(&name.to_lowercase()).cloned()
}

/// Remove a function from the registry
#[allow(dead_code)]
pub fn unregister_function(name: &str) -> Option<FunctionDefinition> {
    let mut registry = FUNCTION_REGISTRY.lock();
    registry.remove(&name.to_lowercase())
}

/// List all registered function names
#[allow(dead_code)]
pub fn list_functions() -> Vec<String> {
    let registry = FUNCTION_REGISTRY.lock();
    registry.keys().cloned().collect()
}

/// Check if a function exists
pub fn function_exists(name: &str) -> bool {
    let registry = FUNCTION_REGISTRY.lock();
    registry.contains_key(&name.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::FunctionParameter;

    #[test]
    fn test_register_and_get_function() {
        let func_def = FunctionDefinition {
            name: "test_func".to_string(),
            parameters: vec![],
            return_type: "INTEGER".to_string(),
            language: "PYTHON".to_string(),
            body: "return 42".to_string(),
        };

        register_function(func_def.clone()).unwrap();

        let retrieved = get_function("test_func").unwrap();
        assert_eq!(retrieved.name, "test_func");
        assert_eq!(retrieved.return_type, "INTEGER");
    }
}
