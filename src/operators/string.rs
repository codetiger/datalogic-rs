//! String operators for JSONLogic
//!
//! This module provides functions for string operations

use bumpalo::Bump;
use datavalue_rs::{DataValue, Error, Number, Result};

/// Evaluates a substring operation on a string
///
/// # Arguments
///
/// * `args` - The arguments for the substring operation: [string, start, length?]
/// * `arena` - The arena allocator
///
/// # Returns
///
/// A substring of the input string
pub fn evaluate_substring<'a>(
    args: &[DataValue<'a>],
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    if args.is_empty() {
        return Err(Error::Custom(
            "Substring requires at least a string and start position".to_string(),
        ));
    }

    // Get the source string, coercing to string if necessary
    let source_str = match &args[0] {
        DataValue::String(s) => s,
        DataValue::Number(Number::Integer(i)) => {
            return Ok(arena.alloc(DataValue::String(arena.alloc_str(&i.to_string()))))
        }
        DataValue::Number(Number::Float(f)) => {
            return Ok(arena.alloc(DataValue::String(arena.alloc_str(&f.to_string()))))
        }
        DataValue::Bool(b) => {
            let s = if *b { "true" } else { "false" };
            return Ok(arena.alloc(DataValue::String(arena.alloc_str(s))));
        }
        DataValue::Null => return Ok(arena.alloc(DataValue::String(arena.alloc_str("null")))),
        DataValue::DateTime(dt) => {
            return Ok(arena.alloc(DataValue::String(arena.alloc_str(&dt.to_string()))))
        }
        DataValue::Duration(d) => {
            return Ok(arena.alloc(DataValue::String(arena.alloc_str(&d.to_string()))))
        }
        DataValue::Array(_) | DataValue::Object(_) => {
            return Err(Error::Custom(
                "Substring first argument must be a scalar value".to_string(),
            ));
        }
    };

    // Return empty string for empty inputs
    if source_str.is_empty() {
        return Ok(arena.alloc(DataValue::String(arena.alloc_str(""))));
    }

    if args.len() < 2 {
        return Ok(arena.alloc(DataValue::String(arena.alloc_str(source_str))));
    }

    // Get the start position
    let start: i64 = match &args[1] {
        DataValue::Number(Number::Integer(i)) => *i,
        DataValue::Number(Number::Float(f)) => *f as i64,
        _ => {
            return Err(Error::Custom(
                "Substring start position must be a number".to_string(),
            ));
        }
    };

    let source_chars: Vec<char> = source_str.chars().collect();
    let source_len = source_chars.len();

    // Handle negative start index (count from end)
    let start_idx = if start < 0 {
        let abs_start = start.unsigned_abs() as usize;
        source_len.saturating_sub(abs_start)
    } else {
        start as usize
    };

    // If start is beyond string length, return empty string
    if start_idx >= source_len {
        return Ok(arena.alloc(DataValue::String(arena.alloc_str(""))));
    }

    // Get the length parameter if provided, otherwise take the rest of the string
    let end_idx = if args.len() > 2 {
        match &args[2] {
            DataValue::Number(Number::Integer(i)) => {
                let length = *i;
                if length >= 0 {
                    // Positive length means take length characters from start
                    std::cmp::min(start_idx + length as usize, source_len)
                } else {
                    // Negative length means go to (length) from the end of string
                    let abs_length = length.unsigned_abs() as usize;
                    if abs_length > source_len {
                        // If negative end is before start of string, use start
                        start_idx
                    } else {
                        // End position is (source_len - abs_length)
                        let end_from_back = source_len - abs_length;
                        if end_from_back <= start_idx {
                            // If the calculated end position is before or at the start, return empty string
                            start_idx
                        } else {
                            end_from_back
                        }
                    }
                }
            }
            DataValue::Number(Number::Float(f)) => {
                let length = *f as i64;
                if length >= 0 {
                    // Positive length means take length characters from start
                    std::cmp::min(start_idx + length as usize, source_len)
                } else {
                    // Negative length means go to (length) from the end of string
                    let abs_length = length.unsigned_abs() as usize;
                    if abs_length > source_len {
                        // If negative end is before start of string, use start
                        start_idx
                    } else {
                        // End position is (source_len - abs_length)
                        let end_from_back = source_len - abs_length;
                        if end_from_back <= start_idx {
                            // If the calculated end position is before or at the start, return empty string
                            start_idx
                        } else {
                            end_from_back
                        }
                    }
                }
            }
            _ => {
                return Err(Error::Custom(
                    "Substring length must be a number".to_string(),
                ));
            }
        }
    } else {
        // If no length specified, take rest of string
        source_len
    };

    // Extract the substring from start_idx to end_idx
    let result = if end_idx > start_idx {
        source_chars[start_idx..end_idx].iter().collect::<String>()
    } else {
        // Empty string if end is before or at start
        String::new()
    };

    // Allocate the result string in the arena
    Ok(arena.alloc(DataValue::String(arena.alloc_str(&result))))
}
