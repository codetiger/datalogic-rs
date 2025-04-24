//! Miscellaneous operators for JSONLogic
//!
//! This module provides implementations for utility operators like
//! missing, missing_some, and exists for data validation

use bumpalo::Bump;
use datavalue_rs::{DataValue, Number, Result};

use crate::DataValueExt;

/// Evaluates the "missing" operator using direct arguments instead of a token
///
/// # Arguments
///
/// * `args` - Vector of DataValues containing the keys to check
/// * `data` - The data context to check against
/// * `arena` - The arena allocator
///
/// # Returns
///
/// An array of missing keys
pub fn evaluate_missing_args<'a>(
    args: &[DataValue<'a>],
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    if args.is_empty() {
        // No keys to check, so none are missing
        return Ok(arena.alloc(DataValue::Array(&[])));
    }

    let mut missing_keys = Vec::new();

    // Process args, which could be individual keys or nested arrays of keys
    for arg in args.iter() {
        match arg {
            DataValue::String(key_str) => {
                if !data.key_exists(key_str) {
                    missing_keys.push(arg.clone());
                }
            }
            DataValue::Array(keys) => {
                // Handle the case where we have an array of keys (e.g., from merge operator)
                for key in keys.iter() {
                    match key {
                        DataValue::String(key_str) => {
                            if !data.key_exists(key_str) {
                                missing_keys.push(key.clone());
                            }
                        }
                        _ => {
                            // If the key is not a string, treat as missing
                            missing_keys.push(key.clone());
                        }
                    }
                }
            }
            _ => {
                // If the key is not a string or array, treat as missing
                missing_keys.push(arg.clone());
            }
        }
    }

    Ok(arena.alloc(DataValue::Array(arena.alloc_slice_fill_iter(missing_keys))))
}

/// Evaluates the "missing_some" operator using direct arguments instead of a token
///
/// # Arguments
///
/// * `args` - Vector of DataValues containing the minimum count and keys to check
/// * `data` - The data context to check against
/// * `arena` - The arena allocator
///
/// # Returns
///
/// An array of missing keys if minimum count is not met, or empty array if it is
pub fn evaluate_missing_some_args<'a>(
    args: &[DataValue<'a>],
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    if args.len() < 2 {
        return Err(datavalue_rs::Error::Custom(
            "missing_some operator requires at least 2 arguments".to_string(),
        ));
    }

    // First argument should be an integer with the minimum number of required keys
    let min_required = match &args[0] {
        DataValue::Number(n) => match n {
            Number::Integer(i) => Ok(*i),
            Number::Float(f) => {
                if f.fract() == 0.0 {
                    Ok(*f as i64)
                } else {
                    Err(datavalue_rs::Error::Custom(
                        "missing_some minimum must be an integer".to_string(),
                    ))
                }
            }
        }?,
        _ => {
            return Err(datavalue_rs::Error::Custom(
                "missing_some minimum must be a number".to_string(),
            ));
        }
    };

    // Second argument should be keys to check
    let keys_arg = &args[1];

    // Collect all keys to check, handling potential nested arrays
    let mut keys = Vec::new();
    let mut missing_keys = Vec::new();

    match keys_arg {
        DataValue::Array(key_array) => {
            // Process each key or nested array of keys
            for key in key_array.iter() {
                match key {
                    DataValue::String(key_str) => {
                        keys.push(key);
                        if !data.key_exists(key_str) {
                            missing_keys.push(key.clone());
                        }
                    }
                    DataValue::Array(nested_keys) => {
                        // Handle nested arrays (e.g., from merge operator)
                        for nested_key in nested_keys.iter() {
                            match nested_key {
                                DataValue::String(nested_key_str) => {
                                    keys.push(nested_key);
                                    if !data.key_exists(nested_key_str) {
                                        missing_keys.push(nested_key.clone());
                                    }
                                }
                                _ => {
                                    // Non-string keys are considered missing
                                    keys.push(nested_key);
                                    missing_keys.push(nested_key.clone());
                                }
                            }
                        }
                    }
                    _ => {
                        // Non-string keys are considered missing
                        keys.push(key);
                        missing_keys.push(key.clone());
                    }
                }
            }
        }
        _ => {
            return Err(datavalue_rs::Error::Custom(
                "missing_some second argument must be an array".to_string(),
            ));
        }
    }

    // Check if enough keys are present
    let total_keys = keys.len() as i64;
    let missing_count = missing_keys.len() as i64;
    let present_count = total_keys - missing_count;

    if present_count >= min_required {
        // Enough keys are present, return empty array
        return Ok(arena.alloc(DataValue::Array(&[])));
    }

    // Not enough keys are present, return missing keys
    Ok(arena.alloc(DataValue::Array(arena.alloc_slice_fill_iter(missing_keys))))
}

/// Evaluates the "exists" operator using direct arguments instead of a token
///
/// # Arguments
///
/// * `args` - Vector of DataValues containing the variable path to check
/// * `data` - The data context to check against
/// * `arena` - The arena allocator
///
/// # Returns
///
/// Boolean true if the variable exists, false otherwise
pub fn evaluate_exists_args<'a>(
    args: &[DataValue<'a>],
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    if args.is_empty() {
        return Ok(arena.alloc(DataValue::Bool(false)));
    }

    // Get the variable path to check
    let var_path = &args[0];

    match var_path {
        DataValue::String(path) => {
            // Check if the path exists in data
            let exists = data.key_exists(path);
            Ok(arena.alloc(DataValue::Bool(exists)))
        }
        DataValue::Array(path_parts) => {
            // For array paths, traverse the data
            let mut current = Some(data);

            for part in path_parts.iter() {
                match part {
                    DataValue::String(key) => {
                        current = current.and_then(|ctx| ctx.get(key));
                    }
                    DataValue::Number(Number::Integer(i)) => {
                        let idx = *i as usize;
                        current = current.and_then(|ctx| ctx.get_index(idx));
                    }
                    _ => {
                        return Err(datavalue_rs::Error::Custom(
                            "Path parts must be strings or integers".to_string(),
                        ));
                    }
                }

                if current.is_none() {
                    break;
                }
            }

            Ok(arena.alloc(DataValue::Bool(current.is_some())))
        }
        _ => Ok(arena.alloc(DataValue::Bool(false))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datavalue_rs::Number;

    #[test]
    fn test_missing() {
        let arena = Bump::new();

        // Create test data with some fields
        let data_str = r#"{"a": 1, "c": "hello"}"#;
        let data = DataValue::from_str(&arena, data_str).unwrap();

        // Test missing fields
        let data_str = r#"["a", "b", "c", "d"]"#;
        let required_keys = DataValue::from_str(&arena, data_str).unwrap();

        let result = evaluate_missing_args(&[required_keys], &data, &arena).unwrap();

        // Should return ["b", "d"]
        if let DataValue::Array(missing) = result {
            assert_eq!(missing.len(), 2);
            assert!(missing.contains(&DataValue::String("b")));
            assert!(missing.contains(&DataValue::String("d")));
        } else {
            panic!("Expected array result");
        }
    }

    #[test]
    fn test_missing_some() {
        let arena = Bump::new();

        // Create test data with some fields
        let data_str = r#"{"a": 1, "c": "hello"}"#;
        let data = DataValue::from_str(&arena, data_str).unwrap();

        // Test with min_needed = 3 (not met)
        let data_str = r#"["a", "b", "c", "d"]"#;
        let required_keys = DataValue::from_str(&arena, data_str).unwrap();

        let result = evaluate_missing_some_args(
            &[DataValue::Number(Number::Integer(3)), required_keys.clone()],
            &data,
            &arena,
        )
        .unwrap();

        // Should return ["b", "d"] because we only found 2 keys but needed 3
        if let DataValue::Array(missing) = result {
            assert_eq!(missing.len(), 2);
            assert!(missing.contains(&DataValue::String("b")));
            assert!(missing.contains(&DataValue::String("d")));
        } else {
            panic!("Expected array result");
        }

        let result = evaluate_missing_some_args(
            &[DataValue::Number(Number::Integer(2)), required_keys.clone()],
            &data,
            &arena,
        )
        .unwrap();

        // Should return [] because we found 2 keys and needed 2
        if let DataValue::Array(missing) = result {
            assert_eq!(missing.len(), 0);
        } else {
            panic!("Expected array result");
        }
    }

    #[test]
    fn test_exists() {
        let arena = Bump::new();

        // Create test data with nested fields
        let data_str = r#"{"a": 1, "b": {"c": "hello"}}"#;
        let data = DataValue::from_str(&arena, data_str).unwrap();

        // Test existing field
        let result = evaluate_exists_args(&[DataValue::String("a")], &data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(true));

        // Test non-existing field
        let result = evaluate_exists_args(&[DataValue::String("z")], &data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(false));

        // Test nested path as array
        let data_str = r#"["b", "c"]"#;
        let path = DataValue::from_str(&arena, data_str).unwrap();

        let result = evaluate_exists_args(&[path], &data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(true));

        // Test non-existing nested path
        let data_str = r#"["b", "d"]"#;
        let path = DataValue::from_str(&arena, data_str).unwrap();

        let result = evaluate_exists_args(&[path], &data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(false));
    }
}
