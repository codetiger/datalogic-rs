//! Collection operators for JSONLogic
//!
//! This module provides functions for evaluating collection operations
//! like filter, map, all, some, and none.

use bumpalo::Bump;
use datavalue_rs::{helpers, DataValue, Error, Number, Result};

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

/// Evaluates a 'cat' operation (string concatenation)
///
/// # Arguments
///
/// * `args` - The token containing the arguments
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// A string concatenation of all arguments
pub fn evaluate_cat<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let mut result = String::new();

    match args {
        Token::Array(tokens) => {
            for token in tokens {
                let value = evaluate(token, data, arena)?;
                match value {
                    DataValue::String(s) => result.push_str(s),
                    DataValue::Number(Number::Integer(i)) => result.push_str(&i.to_string()),
                    DataValue::Number(Number::Float(f)) => result.push_str(&f.to_string()),
                    _ => {
                        return Err(Error::Custom(
                            "Cat operator requires string or number arguments".to_string(),
                        ))
                    }
                }
            }
        }
        _ => {
            let args = evaluate(args, data, arena)?;

            match args {
                DataValue::String(s) => result.push_str(s),
                DataValue::Number(Number::Integer(i)) => result.push_str(&i.to_string()),
                DataValue::Number(Number::Float(f)) => result.push_str(&f.to_string()),
                DataValue::Array(items) => {
                    for item in items.iter() {
                        match item {
                            DataValue::String(s) => result.push_str(s),
                            DataValue::Number(Number::Integer(i)) => {
                                result.push_str(&i.to_string())
                            }
                            DataValue::Number(Number::Float(f)) => result.push_str(&f.to_string()),
                            _ => {
                                return Err(Error::Custom(
                                    "Cat operator requires string or number arguments".to_string(),
                                ))
                            }
                        }
                    }
                }
                _ => {
                    return Err(Error::Custom(
                        "Cat operator requires string or number arguments".to_string(),
                    ))
                }
            }
        }
    }

    // Allocate the result string in the arena
    Ok(arena.alloc(DataValue::String(arena.alloc_str(&result))))
}

/// Evaluates a 'merge' operation (array flattening or string concatenation)
///
/// # Arguments
///
/// * `args` - The token containing the arguments
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// A merged array or concatenated string depending on the input types
pub fn evaluate_merge<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    match args {
        Token::Array(tokens) => {
            if tokens.is_empty() {
                // Empty arguments, return empty array
                return Ok(arena.alloc(DataValue::Array(&[])));
            }

            // First collect all the evaluated values
            let mut values = Vec::new();
            for token in tokens {
                let value = evaluate(token, data, arena)?;
                values.push(value);
            }

            // Check if we're dealing with all strings (for special string concatenation case)
            let mut all_strings = true;

            // Prepare the merged items
            let mut merged_items = Vec::new();

            // Process each value
            for value in values {
                // Check if value is a string for our all_strings tracking
                if !matches!(value, DataValue::String(_)) {
                    all_strings = false;
                }

                // Handle the value based on its type
                if let DataValue::Array(items) = value {
                    // For arrays, flatten by adding each item individually
                    for item in items.iter() {
                        if !matches!(item, DataValue::String(_)) {
                            all_strings = false;
                        }

                        merged_items.push(item.clone());
                    }
                } else {
                    // For non-arrays, add the value as is
                    merged_items.push(value.clone());
                }
            }

            // Special case: if all values were strings, concatenate them
            if all_strings {
                let mut result = String::new();
                for item in merged_items {
                    if let DataValue::String(s) = item {
                        result.push_str(s);
                    }
                }

                // Return the concatenated string
                return Ok(arena.alloc(DataValue::String(arena.alloc_str(&result))));
            }

            Ok(arena.alloc(DataValue::Array(arena.alloc(merged_items))))
        }
        _ => {
            let args = evaluate(args, data, arena)?;

            match args {
                DataValue::Array(items) => {
                    let mut merged_items = Vec::new();
                    for item in items.iter() {
                        merged_items.push(item.clone());
                    }
                    Ok(arena.alloc(DataValue::Array(arena.alloc(merged_items))))
                }
                _ => Ok(arena.alloc(DataValue::Array(
                    arena.alloc_slice_fill_iter(vec![args.clone()]),
                ))),
            }
        }
    }
}

/// Evaluates a 'reduce' operation on an array (equivalent to JavaScript's Array.reduce)
///
/// # Arguments
///
/// * `args` - The token containing the arguments (array, reducer function, and optional initial value)
/// * `data` - The data context
/// * `arena` - The arena allocator
///
/// # Returns
///
/// A single value resulting from the reduction operation
pub fn evaluate_reduce<'a>(
    args: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    match args {
        Token::Array(tokens) => {
            // Reduce requires at least two arguments: array and reducer function
            if tokens.len() < 2 {
                return Err(Error::Custom(
                    "Reduce requires an array and a reducer function".to_string(),
                ));
            }

            // Get the source array
            let array = evaluate(&tokens[0], data, arena)?;

            // Get the reducer function
            let reducer_fn = &tokens[1];

            // If array is empty and no initial value, return null
            if let DataValue::Array(items) = array {
                if items.is_empty() {
                    if tokens.len() < 3 {
                        return Err(Error::Custom(
                            "Reduce of empty array with no initial value".to_string(),
                        ));
                    } else {
                        // Return initial value when array is empty
                        return evaluate(&tokens[2], data, arena);
                    }
                }

                // Determine initial accumulator and starting index
                let (mut accumulator, start_idx) = if tokens.len() > 2 {
                    // Use provided initial value
                    (evaluate(&tokens[2], data, arena)?.clone(), 0)
                } else {
                    // Use first array element as initial value
                    (items[0].clone(), 1)
                };

                // Apply the reducer to each element
                for i in start_idx..items.len() {
                    // Create the context entries as a Vec of tuples
                    let context_entries = arena.alloc([
                        ("accumulator", accumulator.clone()),
                        ("current", items[i].clone()),
                        ("index", helpers::int(i as i64)),
                        ("array", array.clone()),
                    ]);

                    // Build a context with {accumulator, current, index, array}
                    let context_data = arena.alloc(DataValue::Object(context_entries));

                    // Evaluate the reducer with this context
                    let result = evaluate(reducer_fn, context_data, arena)?;

                    // Update the accumulator with the result
                    accumulator = result.clone();
                }

                // Return the final accumulated value
                Ok(arena.alloc(accumulator))
            } else {
                // If not an array, check if we have an initial value
                if tokens.len() > 2 {
                    // Return initial value when input is not an array
                    evaluate(&tokens[2], data, arena)
                } else {
                    // If not an array and no initial value, error
                    Err(Error::Custom(
                        "Reduce requires an array as first argument".to_string(),
                    ))
                }
            }
        }
        _ => Err(Error::Custom(
            "Reduce operator requires array of arguments".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{OperatorType, Token};
    use crate::DataLogic;
    use bumpalo::Bump;
    use datavalue_rs::{DataValue, Number};

    #[test]
    fn test_reduce_operator() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a test array
        let array_items = vec![
            DataValue::Number(Number::Integer(1)),
            DataValue::Number(Number::Integer(2)),
            DataValue::Number(Number::Integer(3)),
            DataValue::Number(Number::Integer(4)),
        ];
        let array_data = DataValue::Array(arena.alloc_slice_fill_iter(array_items));
        let array = Box::new(Token::Literal(array_data.clone()));

        // Create a simple reducer function that sums elements
        let reducer_fn = Box::new(Token::Operator {
            op_type: OperatorType::Add,
            args: Box::new(Token::Array(vec![
                Box::new(Token::Variable {
                    path: arena.alloc(DataValue::String("accumulator")),
                    default: None,
                    scope_jump: None,
                }),
                Box::new(Token::Variable {
                    path: arena.alloc(DataValue::String("current")),
                    default: None,
                    scope_jump: None,
                }),
            ])),
        });

        // Create initial value
        let initial = Box::new(Token::Literal(DataValue::Number(Number::Integer(0))));

        // Create the arguments array for reduce
        let tokens = vec![array, reducer_fn, initial];
        let args = Token::Array(tokens);

        // Test manually by calling the evaluate_reduce function
        let result = evaluate_reduce(&args, data, &arena).unwrap();

        // Verify the result (should be 10 = 0 + 1 + 2 + 3 + 4)
        assert_eq!(*result, DataValue::Number(Number::Integer(10)));
    }

    #[test]
    fn test_reduce_with_no_initial_value() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a test array
        let array_items = vec![
            DataValue::Number(Number::Integer(1)),
            DataValue::Number(Number::Integer(2)),
            DataValue::Number(Number::Integer(3)),
            DataValue::Number(Number::Integer(4)),
        ];
        let array_data = DataValue::Array(arena.alloc_slice_fill_iter(array_items));
        let array = Box::new(Token::Literal(array_data.clone()));

        // Create a simple reducer function that sums elements
        let reducer_fn = Box::new(Token::Operator {
            op_type: OperatorType::Add,
            args: Box::new(Token::Array(vec![
                Box::new(Token::Variable {
                    path: arena.alloc(DataValue::String("accumulator")),
                    default: None,
                    scope_jump: None,
                }),
                Box::new(Token::Variable {
                    path: arena.alloc(DataValue::String("current")),
                    default: None,
                    scope_jump: None,
                }),
            ])),
        });

        // Create the arguments array for reduce
        let tokens = vec![array, reducer_fn];
        let args = Token::Array(tokens);

        // Test manually by calling the evaluate_reduce function
        let result = evaluate_reduce(&args, data, &arena).unwrap();

        // Verify the result (should be 10 = 1 + 2 + 3 + 4)
        assert_eq!(*result, DataValue::Number(Number::Integer(10)));
    }

    #[test]
    fn test_reduce_jsonlogic_syntax() {
        // Initialize the DataLogic engine
        let logic = DataLogic::new();

        // Create a JSONLogic expression for reduce operation
        let rule_str = r#"{
            "reduce": [
                [1, 2, 3, 4, 5],
                {"+": [{"var": "accumulator"}, {"var": "current"}]},
                0
            ]
        }"#;

        // Parse the rule
        let rule = logic.parse_logic(rule_str, None).unwrap();

        // Create data context (not needed for this test)
        let data = logic.parse_data("null").unwrap();

        // Evaluate the rule
        let result = logic.evaluate(&rule, &data).unwrap();

        // Verify the result (should be 15 = 0 + 1 + 2 + 3 + 4 + 5)
        match result {
            DataValue::Number(Number::Integer(n)) => {
                assert_eq!(n, 15);
            }
            _ => panic!("Expected integer result"),
        }
    }
}
