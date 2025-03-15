//! Conditional operator implementations.
//!
//! This module provides implementations for conditional operators such as if and ternary.

use crate::arena::DataArena;
use crate::logic::error::{LogicError, Result};
use crate::logic::evaluator::evaluate;
use crate::logic::token::Token;
use crate::value::DataValue;

/// Enumeration of conditional operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalOp {
    /// If operator
    If,
    /// Ternary operator
    Ternary,
}

/// Evaluates an if operation.
pub fn eval_if<'a>(
    args: &'a [&'a Token<'a>],
    data: &'a DataValue<'a>,
    arena: &'a DataArena,
) -> Result<&'a DataValue<'a>> {
    // Fast path for invalid arguments
    if args.is_empty() {
        return Ok(arena.null_value());
    }
    
    // Process arguments in pairs (condition, value)
    let mut i = 0;
    while i + 1 < args.len() {
        // Evaluate the condition
        let condition = evaluate(args[i], data, arena)?;
        
        // If the condition is true, return the value
        if condition.coerce_to_bool() {
            return evaluate(args[i + 1], data, arena);
        }
        
        // Move to the next pair
        i += 2;
    }
    
    // If there's an odd number of arguments, the last one is the "else" value
    if i < args.len() {
        return evaluate(args[i], data, arena);
    }
    
    // No conditions matched and no else value
    Ok(arena.null_value())
}

/// Evaluates a ternary operation.
pub fn eval_ternary<'a>(
    args: &'a [&'a Token<'a>],
    data: &'a DataValue<'a>,
    arena: &'a DataArena,
) -> Result<&'a DataValue<'a>> {
    // Fast path for invalid arguments
    if args.len() != 3 {
        return Err(LogicError::OperatorError {
            operator: "?:".to_string(),
            reason: format!("Expected 3 arguments, got {}", args.len()),
        });
    }
    
    // Evaluate the condition
    let condition = evaluate(args[0], data, arena)?;
    
    // Return the appropriate branch based on the condition
    if condition.coerce_to_bool() {
        evaluate(args[1], data, arena)
    } else {
        evaluate(args[2], data, arena)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::parser::parse_str;
    use crate::value::FromJson;
    use serde_json::json;

    #[test]
    fn test_evaluate_if() {
        let arena = DataArena::new();
        let data_json = json!({
            "temp": 75
        });
        let data = DataValue::from_json(&data_json, &arena);
        
        // Parse and evaluate an if expression
        let token = parse_str(r#"{
            "if": [
                {">": [{"var": "temp"}, 80]},
                "hot",
                {"<": [{"var": "temp"}, 70]},
                "cold",
                "pleasant"
            ]
        }"#, &arena).unwrap();
        
        let result = crate::logic::evaluator::evaluate(token, &data, &arena).unwrap();
        assert_eq!(result.as_str(), Some("pleasant"));
    }

    #[test]
    fn test_evaluate_ternary() {
        let arena = DataArena::new();
        let data_json = json!({
            "age": 25
        });
        let data = DataValue::from_json(&data_json, &arena);
        
        // Create tokens for the ternary operation
        let condition = parse_str(r#"{">": [{"var": "age"}, 21]}"#, &arena).unwrap();
        let true_result = parse_str(r#""adult""#, &arena).unwrap();
        let false_result = parse_str(r#""minor""#, &arena).unwrap();
        
        // Manually evaluate the ternary operation
        let condition_value = evaluate(condition, &data, &arena).unwrap();
        let result = if condition_value.coerce_to_bool() {
            evaluate(true_result, &data, &arena).unwrap()
        } else {
            evaluate(false_result, &data, &arena).unwrap()
        };
        
        assert_eq!(result.as_str(), Some("adult"));
    }
} 