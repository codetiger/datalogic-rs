//! Comparison operators for JSONLogic
//!
//! This module provides implementation of comparison operators.

use bumpalo::Bump;
use datavalue_rs::{DataValue, Number, Result};

use crate::parser::Token;
use crate::value::{loose_equals, strict_equals};
use crate::{engine, DataValueExt};

/// Evaluates an equal (==) comparison
pub fn evaluate_equal<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let values = get_arg_values(args, data, arena)?;
    println!("values: {:?}", values);

    if values.len() < 2 {
        return Ok(arena.alloc(DataValue::Bool(true))); // One or zero values is always true
    }

    // Perform chained equality (a == b && b == c && ...)
    for i in 0..values.len() - 1 {
        if !loose_equals(values[i], values[i + 1]) {
            return Ok(arena.alloc(DataValue::Bool(false)));
        }
    }

    Ok(arena.alloc(DataValue::Bool(true)))
}

/// Evaluates a strict equal (===) comparison
pub fn evaluate_strict_equal<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let values = get_arg_values(args, data, arena)?;
    println!("values: {:?}", values);
    
    if values.len() < 2 {
        return Ok(arena.alloc(DataValue::Bool(true))); // One or zero values is always true
    }

    // Perform chained strict equality (a === b && b === c && ...)
    for i in 0..values.len() - 1 {
        if !strict_equals(values[i], values[i + 1]) {
            return Ok(arena.alloc(DataValue::Bool(false)));
        }
    }

    Ok(arena.alloc(DataValue::Bool(true)))
}

/// Evaluates a not equal (!=) comparison
pub fn evaluate_not_equal<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let values = get_arg_values(args, data, arena)?;

    if values.len() < 2 {
        return Ok(arena.alloc(DataValue::Bool(false))); // One or zero values is always false (negation of equal)
    }

    // Perform chained inequality (a != b && b != c && ...)
    for i in 0..values.len() - 1 {
        if loose_equals(values[i], values[i + 1]) {
            return Ok(arena.alloc(DataValue::Bool(false)));
        }
    }

    Ok(arena.alloc(DataValue::Bool(true)))
}

/// Evaluates a strict not equal (!==) comparison
pub fn evaluate_strict_not_equal<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let values = get_arg_values(args, data, arena)?;

    if values.len() < 2 {
        return Ok(arena.alloc(DataValue::Bool(false))); // One or zero values is always false (negation of strict equal)
    }

    // Perform chained strict inequality (a !== b && b !== c && ...)
    for i in 0..values.len() - 1 {
        if strict_equals(values[i], values[i + 1]) {
            return Ok(arena.alloc(DataValue::Bool(false)));
        }
    }

    Ok(arena.alloc(DataValue::Bool(true)))
}

/// Evaluates a greater than (>) comparison
pub fn evaluate_gt<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let values = get_arg_values(args, data, arena)?;

    if values.len() < 2 {
        return Ok(arena.alloc(DataValue::Bool(false))); // One or zero values can't be compared
    }

    // Perform chained greater than (a > b && b > c && ...)
    for i in 0..values.len() - 1 {
        let (a, b) = (
            values[i].coerce_to_number(),
            values[i + 1].coerce_to_number(),
        );
        let a_val = if let DataValue::Number(Number::Integer(i)) = a {
            i as f64
        } else if let DataValue::Number(Number::Float(f)) = a {
            f
        } else {
            return Ok(arena.alloc(DataValue::Bool(false)));
        };
        let b_val = if let DataValue::Number(Number::Integer(i)) = b {
            i as f64
        } else if let DataValue::Number(Number::Float(f)) = b {
            f
        } else {
            return Ok(arena.alloc(DataValue::Bool(false)));
        };
        if a_val <= b_val {
            return Ok(arena.alloc(DataValue::Bool(false)));
        }
    }

    Ok(arena.alloc(DataValue::Bool(true)))
}

/// Evaluates a less than (<) comparison
pub fn evaluate_lt<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let values = get_arg_values(args, data, arena)?;

    if values.len() < 2 {
        return Ok(arena.alloc(DataValue::Bool(false))); // One or zero values can't be compared
    }

    // Perform chained less than (a < b && b < c && ...)
    for i in 0..values.len() - 1 {
        let (a, b) = (
            values[i].coerce_to_number(),
            values[i + 1].coerce_to_number(),
        );
        let a_val = if let DataValue::Number(Number::Integer(i)) = a {
            i as f64
        } else if let DataValue::Number(Number::Float(f)) = a {
            f
        } else {
            return Ok(arena.alloc(DataValue::Bool(false)));
        };
        let b_val = if let DataValue::Number(Number::Integer(i)) = b {
            i as f64
        } else if let DataValue::Number(Number::Float(f)) = b {
            f
        } else {
            return Ok(arena.alloc(DataValue::Bool(false)));
        };
        if a_val >= b_val {
            return Ok(arena.alloc(DataValue::Bool(false)));
        }
    }

    Ok(arena.alloc(DataValue::Bool(true)))
}

/// Evaluates a greater than or equal (>=) comparison
pub fn evaluate_gte<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let values = get_arg_values(args, data, arena)?;

    if values.len() < 2 {
        return Ok(arena.alloc(DataValue::Bool(true))); // One or zero values is always true
    }

    // Perform chained greater than or equal (a >= b && b >= c && ...)
    for i in 0..values.len() - 1 {
        let (a, b) = (
            values[i].coerce_to_number(),
            values[i + 1].coerce_to_number(),
        );
        let a_val = if let DataValue::Number(Number::Integer(i)) = a {
            i as f64
        } else if let DataValue::Number(Number::Float(f)) = a {
            f
        } else {
            return Ok(arena.alloc(DataValue::Bool(false)));
        };
        let b_val = if let DataValue::Number(Number::Integer(i)) = b {
            i as f64
        } else if let DataValue::Number(Number::Float(f)) = b {
            f
        } else {
            return Ok(arena.alloc(DataValue::Bool(false)));
        };
        if a_val < b_val {
            return Ok(arena.alloc(DataValue::Bool(false)));
        }
    }

    Ok(arena.alloc(DataValue::Bool(true)))
}

/// Evaluates a less than or equal (<=) comparison
pub fn evaluate_lte<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let values = get_arg_values(args, data, arena)?;

    if values.len() < 2 {
        return Ok(arena.alloc(DataValue::Bool(true))); // One or zero values is always true
    }

    // Perform chained less than or equal (a <= b && b <= c && ...)
    for i in 0..values.len() - 1 {
        let (a, b) = (
            values[i].coerce_to_number(),
            values[i + 1].coerce_to_number(),
        );
        let a_val = if let DataValue::Number(Number::Integer(i)) = a {
            i as f64
        } else if let DataValue::Number(Number::Float(f)) = a {
            f
        } else {
            return Ok(arena.alloc(DataValue::Bool(false)));
        };
        let b_val = if let DataValue::Number(Number::Integer(i)) = b {
            i as f64
        } else if let DataValue::Number(Number::Float(f)) = b {
            f
        } else {
            return Ok(arena.alloc(DataValue::Bool(false)));
        };
        if a_val > b_val {
            return Ok(arena.alloc(DataValue::Bool(false)));
        }
    }

    Ok(arena.alloc(DataValue::Bool(true)))
}

/// Helper function to extract argument values
fn get_arg_values<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<Vec<&'a DataValue<'a>>> {
    match args {
        Token::Array(items) => {
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                // Fully evaluate each item recursively
                let value = engine::evaluate(item, data, arena)?;
                values.push(value);
            }
            Ok(values)
        }
        Token::ArrayLiteral(items) => Ok(items.iter().collect()),
        Token::Operator { .. } => {
            // If we have a nested operator, evaluate it first
            let value = engine::evaluate(args, data, arena)?;
            Ok(vec![value])
        }
        _ => {
            // For any other token type, evaluate it and return as a single-item vector
            let value = engine::evaluate(args, data, arena)?;
            Ok(vec![value])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Token;

    #[test]
    fn test_equal() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Test equal values: [1, 1, "1"] (all loose equal)
        let args = Token::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::String("1"),
        ]);

        let result = evaluate_equal(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(true));

        // Test unequal values: [1, 2, 3]
        let args = Token::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(2)),
            DataValue::Number(datavalue_rs::Number::Integer(3)),
        ]);

        let result = evaluate_equal(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(false));
    }

    #[test]
    fn test_strict_equal() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Test strictly equal values: [1, 1, 1]
        let args = Token::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(1)),
        ]);

        let result = evaluate_strict_equal(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(true));

        // Test not strictly equal values: [1, 1, "1"]
        let args = Token::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::String("1"),
        ]);

        let result = evaluate_strict_equal(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(false));
    }

    #[test]
    fn test_greater_than() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Test decreasing sequence: [5, 4, 3, 2, 1]
        let args = Token::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(5)),
            DataValue::Number(datavalue_rs::Number::Integer(4)),
            DataValue::Number(datavalue_rs::Number::Integer(3)),
            DataValue::Number(datavalue_rs::Number::Integer(2)),
            DataValue::Number(datavalue_rs::Number::Integer(1)),
        ]);

        let result = evaluate_gt(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(true));

        // Test non-decreasing sequence: [5, 5, 4, 3]
        let args = Token::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(5)),
            DataValue::Number(datavalue_rs::Number::Integer(5)),
            DataValue::Number(datavalue_rs::Number::Integer(4)),
            DataValue::Number(datavalue_rs::Number::Integer(3)),
        ]);

        let result = evaluate_gt(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(false));
    }

    #[test]
    fn test_less_than() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Test increasing sequence: [1, 2, 3, 4, 5]
        let args = Token::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(2)),
            DataValue::Number(datavalue_rs::Number::Integer(3)),
            DataValue::Number(datavalue_rs::Number::Integer(4)),
            DataValue::Number(datavalue_rs::Number::Integer(5)),
        ]);

        let result = evaluate_lt(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(true));

        // Test non-increasing sequence: [1, 1, 2, 3]
        let args = Token::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(2)),
            DataValue::Number(datavalue_rs::Number::Integer(3)),
        ]);

        let result = evaluate_lt(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(false));
    }
}
