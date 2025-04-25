//! Logic operators for JSONLogic
//!
//! This module provides functions for evaluating logical operations like
//! if, and, or, not, and ?? (null coalescing)

use bumpalo::Bump;
use datavalue_rs::{helpers, DataValue, Result};

use crate::{evaluate, ASTNode, DataValueExt};

/// Evaluates an if-then-else operation
///
/// # Arguments
///
/// * `args` - The token containing the arguments
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// The result of evaluating the if expression
pub fn evaluate_if<'a>(
    args: &'a ASTNode<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    match args {
        ASTNode::Array(tokens) => {
            // If requires at least one argument (the condition)
            if tokens.is_empty() {
                return Ok(arena.alloc(helpers::null()));
            }

            // Process pairs of condition/value with optional default
            let mut i = 0;
            while i < tokens.len() - 1 {
                // Evaluate the condition
                let condition = evaluate(&tokens[i], data, arena)?;

                // If condition is truthy, return the corresponding value
                if condition.is_truthy() {
                    return evaluate(&tokens[i + 1], data, arena);
                }

                // Move to the next condition-value pair
                i += 2;
            }

            // If we have an odd number of arguments, the last one is the default
            if i < tokens.len() {
                return evaluate(&tokens[i], data, arena);
            }

            // No conditions matched and no default
            Ok(arena.alloc(helpers::null()))
        }
        ASTNode::ArrayLiteral(values) => {
            // If requires at least one argument (the condition)
            if values.is_empty() {
                return Ok(arena.alloc(helpers::null()));
            }

            // Process pairs of condition/value with optional default
            let mut i = 0;
            while i < values.len() - 1 {
                // If condition is truthy, return the corresponding value
                if values[i].is_truthy() {
                    return Ok(&values[i + 1]);
                }

                // Move to the next condition-value pair
                i += 2;
            }

            // If we have an odd number of arguments, the last one is the default
            if i < values.len() {
                return Ok(&values[i]);
            }

            // No conditions matched and no default
            Ok(arena.alloc(helpers::null()))
        }
        _ => Err(datavalue_rs::Error::Custom(
            "If operator requires array of arguments".to_string(),
        )),
    }
}

/// Evaluates a logical AND operation with short-circuit evaluation
///
/// # Arguments
///
/// * `args` - The token containing the arguments
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// The result of the AND operation
pub fn evaluate_and<'a>(
    args: &'a ASTNode<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    match args {
        ASTNode::Array(tokens) => {
            // Empty array returns true (identity element for AND)
            if tokens.is_empty() {
                return Ok(arena.alloc(helpers::null()));
            }

            // Short-circuit: evaluate arguments one at a time
            for token in tokens {
                let value = evaluate(token, data, arena)?;
                if !value.is_truthy() {
                    // Short-circuit: return the first falsy value
                    return Ok(value);
                }
            }

            // All values were truthy, return the last one
            if let Some(last) = tokens.last() {
                evaluate(last, data, arena)
            } else {
                // Should be unreachable due to the empty check above
                Ok(arena.alloc(helpers::null()))
            }
        }
        ASTNode::ArrayLiteral(values) => {
            // Empty array returns true (identity element for AND)
            if values.is_empty() {
                return Ok(arena.alloc(helpers::null()));
            }

            let mut last = None;
            // Short-circuit: evaluate arguments one at a time
            for value in values {
                if !value.is_truthy() {
                    return Ok(value);
                }
                last = Some(value);
            }

            // All values were truthy, return the last one
            if let Some(last) = last {
                Ok(arena.alloc(last))
            } else {
                Ok(arena.alloc(helpers::null()))
            }
        }
        _ => Err(datavalue_rs::Error::Custom(
            "And operator requires array of arguments".to_string(),
        )),
    }
}

/// Evaluates a logical OR operation with short-circuit evaluation
///
/// # Arguments
///
/// * `args` - The token containing the arguments
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// The result of the OR operation
pub fn evaluate_or<'a>(
    args: &'a ASTNode<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    match args {
        ASTNode::Array(tokens) => {
            // Empty array returns false (identity element for OR)
            if tokens.is_empty() {
                return Ok(arena.alloc(helpers::null()));
            }

            // Short-circuit: evaluate arguments one at a time
            for token in tokens {
                let value = evaluate(token, data, arena)?;
                if value.is_truthy() {
                    // Short-circuit: return the first truthy value
                    return Ok(value);
                }
            }

            // No values were truthy, return the last one
            if let Some(last) = tokens.last() {
                evaluate(last, data, arena)
            } else {
                // Should be unreachable due to the empty check above
                Ok(arena.alloc(helpers::null()))
            }
        }
        ASTNode::ArrayLiteral(values) => {
            // Empty array returns false (identity element for OR)
            if values.is_empty() {
                return Ok(arena.alloc(helpers::null()));
            }

            let mut last = None;
            // Short-circuit: evaluate arguments one at a time
            for value in values {
                if value.is_truthy() {
                    return Ok(value);
                }
                last = Some(value);
            }

            // All values were falsy, return the last one
            if let Some(last) = last {
                Ok(arena.alloc(last))
            } else {
                Ok(arena.alloc(helpers::null()))
            }
        }
        _ => Err(datavalue_rs::Error::Custom(
            "Or operator requires array of arguments".to_string(),
        )),
    }
}

/// Evaluates a logical NOT operation
///
/// # Arguments
///
/// * `args` - The token containing the arguments
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// The result of the NOT operation
pub fn evaluate_not<'a>(values: &[DataValue<'a>], arena: &'a Bump) -> Result<&'a DataValue<'a>> {
    match values {
        [] => Ok(arena.alloc(helpers::boolean(true))),
        [DataValue::Array([value])] => Ok(arena.alloc(helpers::boolean(!value.is_truthy()))),
        [value] => Ok(arena.alloc(helpers::boolean(!value.is_truthy()))),
        _ => Err(datavalue_rs::Error::Custom(
            "Not operator requires exactly one argument".to_string(),
        )),
    }
}

/// Evaluates a double negation operation (!!)
///
/// # Arguments
///
/// * `args` - The token containing the arguments
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// The result of the double negation operation
pub fn evaluate_double_bang<'a>(
    values: &[DataValue<'a>],
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    match values {
        [] => Ok(arena.alloc(helpers::boolean(false))),
        [DataValue::Array([value])] => Ok(arena.alloc(helpers::boolean(value.is_truthy()))),
        [value] => Ok(arena.alloc(helpers::boolean(value.is_truthy()))),
        _ => Err(datavalue_rs::Error::Custom(
            "DoubleBang operator requires exactly one argument".to_string(),
        )),
    }
}

/// Evaluates a null coalescing operation (??)
///
/// # Arguments
///
/// * `args` - The token containing the arguments
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// The first non-null value or null if all values are null
pub fn evaluate_null_coalesce<'a>(
    args: &'a ASTNode<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    match args {
        ASTNode::Array(tokens) => {
            // Empty array case
            if tokens.is_empty() {
                return Ok(arena.alloc(helpers::null()));
            }

            // Evaluate each token until we find a non-null value
            for token in tokens {
                let value = evaluate(token, data, arena)?;
                if !matches!(value, DataValue::Null) {
                    return Ok(value);
                }
            }

            // All values were null
            Ok(arena.alloc(helpers::null()))
        }
        _ => {
            // Single argument case, just evaluate it
            evaluate(args, data, arena)
        }
    }
}
