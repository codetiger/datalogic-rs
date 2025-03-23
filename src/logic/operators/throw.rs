//! Throw operator implementation.
//!
//! This module provides the implementation of the throw operator.

use crate::arena::DataArena;
use crate::logic::error::{LogicError, Result};
use crate::logic::evaluator::evaluate;
use crate::logic::token::Token;
use crate::value::DataValue;

/// Evaluates a throw operation.
/// The throw operator throws an error with the provided value.
#[inline]
pub fn eval_throw<'a>(
    args: &'a [&'a Token<'a>],
    data: &'a DataValue<'a>,
    arena: &'a DataArena,
) -> Result<&'a DataValue<'a>> {
    // Check if we have the right number of arguments
    if args.is_empty() {
        return Err(LogicError::InvalidArgumentsError);
    }

    // Evaluate the first argument to get the error value/type
    let error_value = evaluate(args[0], data, arena)?;
    
    // For string values, use them directly as the error type
    if let Some(error_str) = error_value.as_str() {
        return Err(LogicError::thrown_error(error_str));
    }
    
    // Handle object values with a "type" field
    if let Some(obj) = error_value.as_object() {
        for (key, value) in obj {
            if *key == "type" {
                if let Some(type_str) = value.as_str() {
                    return Err(LogicError::thrown_error(type_str));
                }
            }
        }
    }
    
    // For other values, convert to string
    let error_str = if let Some(i) = error_value.as_i64() {
        i.to_string()
    } else if let Some(f) = error_value.as_f64() {
        f.to_string()
    } else if let Some(b) = error_value.as_bool() {
        b.to_string()
    } else if error_value.is_null() {
        "null".to_string()
    } else {
        "Unknown error".to_string()
    };
    
    Err(LogicError::thrown_error(error_str))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::parser::parse_str;
    use crate::value::FromJson;
    use serde_json::json;

    #[test]
    fn test_evaluate_throw_string() {
        let arena = DataArena::new();
        let data = DataValue::null();
        
        // Parse and evaluate a throw expression with a string
        let token = parse_str(r#"{"throw": "hello"}"#, &arena).unwrap();
        
        let result = crate::logic::evaluator::evaluate(token, &data, &arena);
        assert!(result.is_err());
        if let Err(LogicError::ThrownError { r#type: error_type }) = result {
            assert_eq!(error_type, "hello");
        } else {
            panic!("Expected ThrownError, got: {:?}", result);
        }
    }

    #[test]
    fn test_evaluate_throw_object() {
        let arena = DataArena::new();
        let data_json = json!({
            "x": {"type": "Some error"}
        });
        let data = DataValue::from_json(&data_json, &arena);
        
        // Parse and evaluate a throw expression with an object
        let token = parse_str(r#"{"throw": {"val": "x"}}"#, &arena).unwrap();
        
        let result = crate::logic::evaluator::evaluate(token, &data, &arena);
        assert!(result.is_err());
        if let Err(LogicError::ThrownError { r#type: error_type }) = result {
            assert_eq!(error_type, "Some error");
        } else {
            panic!("Expected ThrownError, got: {:?}", result);
        }
    }
} 