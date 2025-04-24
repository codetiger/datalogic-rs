//! Collection operators for JSONLogic
//!
//! This module provides functions for evaluating collection operations
//! like filter, map, all, some, and none.

use bumpalo::Bump;
use datavalue_rs::{helpers, DataValue, Error, Result};

use crate::{evaluate, DataValueExt, Token};


/// Evaluates a filter operation on an array
///
/// # Arguments
///
/// * `args` - The token containing the arguments (array and predicate)
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// A new array with elements that pass the predicate test
pub fn evaluate_filter<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    match args {
        Token::Array(tokens) => {
            // Filter requires at least two arguments: array and predicate
            if tokens.len() < 2 {
                return Err(Error::Custom(
                    "Filter requires an array and a predicate".to_string(),
                ));
            }

            // Get the source array
            let array = evaluate(&tokens[0], data, arena)?;
            
            // Get the predicate
            let predicate = &tokens[1];

            // Create a new array to hold the filtered elements
            let mut filtered = Vec::new();

            // Apply the predicate to each element if it's an array
            // If not an array, treat it as an empty array (return [])
            if let DataValue::Array(items) = array {
                for item in items.iter() {
                    // Evaluate the predicate with this context
                    let result = evaluate(predicate, item, arena)?;

                    // If the predicate is truthy, include the item
                    if result.is_truthy() {
                        filtered.push(item.clone());
                    }
                }
            }
            // For non-array values (including null), return an empty array

            // Allocate the result array in the arena
            Ok(arena.alloc(DataValue::Array(arena.alloc(filtered))))
        }
        _ => Err(Error::Custom(
            "Filter operator requires array of arguments".to_string(),
        )),
    }
}

/// Evaluates a map operation on an array
///
/// # Arguments
///
/// * `args` - The token containing the arguments (array and mapping function)
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// A new array with the mapping function applied to each element
pub fn evaluate_map<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    match args {
        Token::Array(tokens) => {
            // Map requires at least two arguments: array and function
            if tokens.len() < 2 {
                return Err(Error::Custom(
                    "Map requires an array and a mapping function".to_string(),
                ));
            }

            // Get the source array
            let array = evaluate(&tokens[0], data, arena)?;
            
            // Get the mapping function
            let mapping_fn = &tokens[1];

            // Create a new array to hold the mapped elements
            let mut mapped = Vec::new();

            // Apply the function to each element if it's an array
            // If not an array, treat it as an empty array (return [])
            if let DataValue::Array(items) = array {
                for item in items.iter() {
                    // Evaluate the mapping function with this context
                    let result = evaluate(mapping_fn, item, arena)?;
                    mapped.push(result.clone());
                }
            }
            // For non-array values (including null), return an empty array

            // Allocate the result array in the arena
            Ok(arena.alloc(DataValue::Array(arena.alloc(mapped))))
        }
        _ => Err(Error::Custom(
            "Map operator requires array of arguments".to_string(),
        )),
    }
}

/// Evaluates an 'all' operation on an array
///
/// # Arguments
///
/// * `args` - The token containing the arguments (array and predicate)
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// True if all elements pass the predicate test, false otherwise
pub fn evaluate_all<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    match args {
        Token::Array(tokens) => {
            // All requires at least two arguments: array and predicate
            if tokens.len() < 2 {
                return Err(Error::Custom(
                    "All requires an array and a predicate".to_string(),
                ));
            }

            // Get the source array
            let array = evaluate(&tokens[0], data, arena)?;
            
            // Get the predicate
            let predicate = &tokens[1];

            // If it's not an array or is an empty array, return false
            if !array.is_array() || matches!(array, DataValue::Array(items) if items.is_empty()) {
                return Ok(arena.alloc(helpers::boolean(false)));
            }

            // Apply the predicate to each element
            if let DataValue::Array(items) = array {
                for item in items.iter() {
                    // Evaluate the predicate with this context
                    let result = evaluate(predicate, item, arena)?;

                    // If any predicate is falsy, short-circuit and return false
                    if !result.is_truthy() {
                        return Ok(arena.alloc(helpers::boolean(false)));
                    }
                }

                // All predicates passed, return true
                Ok(arena.alloc(helpers::boolean(true)))
            } else {
                // Should never reach here due to the earlier check
                unreachable!()
            }
        }
        _ => Err(Error::Custom(
            "All operator requires array of arguments".to_string(),
        )),
    }
}

/// Evaluates a 'some' operation on an array (equivalent to JavaScript's Array.some)
///
/// # Arguments
///
/// * `args` - The token containing the arguments (array and predicate)
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// True if at least one element passes the predicate test, false otherwise
pub fn evaluate_some<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    match args {
        Token::Array(tokens) => {
            // Some requires at least two arguments: array and predicate
            if tokens.len() < 2 {
                return Err(Error::Custom(
                    "Some requires an array and a predicate".to_string(),
                ));
            }

            // Get the source array
            let array = evaluate(&tokens[0], data, arena)?;
            
            // Get the predicate
            let predicate = &tokens[1];

            // If it's not an array or is an empty array, return false
            if !array.is_array() || matches!(array, DataValue::Array(items) if items.is_empty()) {
                return Ok(arena.alloc(helpers::boolean(false)));
            }

            // Apply the predicate to each element
            if let DataValue::Array(items) = array {
                for item in items.iter() {
                    // Evaluate the predicate with this context
                    let result = evaluate(predicate, item, arena)?;

                    // If any predicate is truthy, short-circuit and return true
                    if result.is_truthy() {
                        return Ok(arena.alloc(helpers::boolean(true)));
                    }
                }

                // No predicate passed, return false
                Ok(arena.alloc(helpers::boolean(false)))
            } else {
                // Should never reach here due to the earlier check
                unreachable!()
            }
        }
        _ => Err(Error::Custom(
            "Some operator requires array of arguments".to_string(),
        )),
    }
}

/// Evaluates a 'none' operation on an array
///
/// # Arguments
///
/// * `args` - The token containing the arguments (array and predicate)
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// True if no elements pass the predicate test, false otherwise
pub fn evaluate_none<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    match args {
        Token::Array(tokens) => {
            // None requires at least two arguments: array and predicate
            if tokens.len() < 2 {
                return Err(Error::Custom(
                    "None requires an array and a predicate".to_string(),
                ));
            }

            // Get the source array
            let array = evaluate(&tokens[0], data, arena)?;
            
            // Get the predicate
            let predicate = &tokens[1];

            // If it's not an array, treat it as an empty array,
            // and since no elements match the predicate (vacuously), return true
            if !array.is_array() {
                return Ok(arena.alloc(helpers::boolean(true)));
            }

            // Empty arrays return true by definition (no elements match)
            if let DataValue::Array(items) = array {
                if items.is_empty() {
                    return Ok(arena.alloc(helpers::boolean(true)));
                }

                // Apply the predicate to each element
                for item in items.iter() {
                    // Evaluate the predicate with this context
                    let result = evaluate(predicate, item, arena)?;

                    // If any predicate is truthy, short-circuit and return false
                    if result.is_truthy() {
                        return Ok(arena.alloc(helpers::boolean(false)));
                    }
                }

                // No predicate passed, return true
                Ok(arena.alloc(helpers::boolean(true)))
            } else {
                // Should never reach here due to the earlier check
                unreachable!()
            }
        }
        _ => Err(Error::Custom(
            "None operator requires array of arguments".to_string(),
        )),
    }
}
