//! Comparison operators for JSONLogic
//!
//! This module provides implementation of comparison operators.

use bumpalo::Bump;
use datavalue_rs::{DataValue, Number, Result};

use crate::parser::ASTNode;
use crate::value::{loose_equals, strict_equals};
use crate::{engine, DataValueExt};

/// Evaluates an equal (==) comparison
pub fn evaluate_equal<'a>(
    args: &'a ASTNode<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let values = get_arg_values(args, data, arena)?;

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
    args: &'a ASTNode<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let values = get_arg_values(args, data, arena)?;

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
    args: &'a ASTNode<'a>,
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
    args: &'a ASTNode<'a>,
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
    args: &'a ASTNode<'a>,
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
    args: &'a ASTNode<'a>,
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
    args: &'a ASTNode<'a>,
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
    args: &'a ASTNode<'a>,
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

/// Evaluates an 'in' operation to check if a value is in an array or a substring is in a string
///
/// # Arguments
///
/// * `args` - The token containing the arguments (value to check, array or string to search in)
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// True if the value is found in the array or if the substring is found in the string
pub fn evaluate_in<'a>(
    args: &'a ASTNode<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let values = get_arg_values(args, data, arena)?;

    if values.len() < 2 {
        return Ok(arena.alloc(DataValue::Bool(false))); // Need at least two values
    }

    let needle = values[0];
    let haystack = values[1];

    // Check if searching in a string
    if let DataValue::String(haystack_str) = haystack {
        // Convert needle to string if it's not already
        let needle_str = match needle {
            DataValue::String(s) => s,
            DataValue::Number(Number::Integer(i)) => {
                return Ok(arena.alloc(DataValue::Bool(haystack_str.contains(&i.to_string()))))
            }
            DataValue::Number(Number::Float(f)) => {
                return Ok(arena.alloc(DataValue::Bool(haystack_str.contains(&f.to_string()))))
            }
            DataValue::Bool(b) => {
                let s = if *b { "true" } else { "false" };
                return Ok(arena.alloc(DataValue::Bool(haystack_str.contains(s))));
            }
            DataValue::Null => {
                return Ok(arena.alloc(DataValue::Bool(haystack_str.contains("null"))))
            }
            DataValue::DateTime(dt) => {
                return Ok(arena.alloc(DataValue::Bool(haystack_str.contains(&dt.to_string()))))
            }
            DataValue::Duration(d) => {
                return Ok(arena.alloc(DataValue::Bool(haystack_str.contains(&d.to_string()))))
            }
            DataValue::Array(_) | DataValue::Object(_) => {
                return Ok(arena.alloc(DataValue::Bool(false))); // Complex types can't be in strings
            }
        };

        return Ok(arena.alloc(DataValue::Bool(haystack_str.contains(needle_str))));
    }

    // Check if searching in an array
    if let DataValue::Array(items) = haystack {
        for item in items.iter() {
            // Use loose equality to check if the needle is in the array
            if loose_equals(needle, item) {
                return Ok(arena.alloc(DataValue::Bool(true)));
            }
        }
        return Ok(arena.alloc(DataValue::Bool(false)));
    }

    // If haystack is neither a string nor an array, return false
    Ok(arena.alloc(DataValue::Bool(false)))
}

/// Helper function to extract argument values
fn get_arg_values<'a>(
    args: &'a ASTNode<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<Vec<&'a DataValue<'a>>> {
    match args {
        ASTNode::Array(items) => {
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                // Fully evaluate each item recursively
                let value = engine::evaluate(item, data, arena)?;
                values.push(value);
            }
            Ok(values)
        }
        ASTNode::ArrayLiteral(items) => Ok(items.iter().collect()),
        ASTNode::Operator { .. } => {
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
    use crate::parser::ASTNode;

    #[test]
    fn test_equal() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Test equal values: [1, 1, "1"] (all loose equal)
        let args = ASTNode::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::String("1"),
        ]);

        let result = evaluate_equal(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(true));

        // Test unequal values: [1, 2, 3]
        let args = ASTNode::ArrayLiteral(vec![
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
        let args = ASTNode::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(1)),
        ]);

        let result = evaluate_strict_equal(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(true));

        // Test not strictly equal values: [1, 1, "1"]
        let args = ASTNode::ArrayLiteral(vec![
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
        let args = ASTNode::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(5)),
            DataValue::Number(datavalue_rs::Number::Integer(4)),
            DataValue::Number(datavalue_rs::Number::Integer(3)),
            DataValue::Number(datavalue_rs::Number::Integer(2)),
            DataValue::Number(datavalue_rs::Number::Integer(1)),
        ]);

        let result = evaluate_gt(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(true));

        // Test non-decreasing sequence: [5, 5, 4, 3]
        let args = ASTNode::ArrayLiteral(vec![
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
        let args = ASTNode::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(2)),
            DataValue::Number(datavalue_rs::Number::Integer(3)),
            DataValue::Number(datavalue_rs::Number::Integer(4)),
            DataValue::Number(datavalue_rs::Number::Integer(5)),
        ]);

        let result = evaluate_lt(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(true));

        // Test non-increasing sequence: [1, 1, 2, 3]
        let args = ASTNode::ArrayLiteral(vec![
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(1)),
            DataValue::Number(datavalue_rs::Number::Integer(2)),
            DataValue::Number(datavalue_rs::Number::Integer(3)),
        ]);

        let result = evaluate_lt(&args, data, &arena).unwrap();
        assert_eq!(*result, DataValue::Bool(false));
    }
}
