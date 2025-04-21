//! Comparison operations for JSONLogic.
//!
//! This module implements comparison operations for DataValue
//! according to JSONLogic rules, including loose and strict equality.

use datavalue_rs::{DataValue, Number};
use std::cmp::Ordering;

/// Compare two DataValues according to JSONLogic rules.
///
/// The comparison follows these rules:
/// - Numbers are compared by their numeric value
/// - Strings are compared lexicographically
/// - Arrays are compared element by element
/// - Objects are compared by their properties
/// - Different types are compared after type conversion (when possible)
///
/// # Examples
///
/// ```
/// use datavalue_rs::{helpers, Bump};
/// use datalogic_rs::value::compare_values;
/// use std::cmp::Ordering;
///
/// let arena = Bump::new();
///
/// // Same type comparisons
/// assert_eq!(compare_values(&helpers::int(1), &helpers::int(2)), Ordering::Less);
/// assert_eq!(compare_values(&helpers::string(&arena, "a"), &helpers::string(&arena, "b")), Ordering::Less);
///
/// // Different type comparisons
/// assert_eq!(compare_values(&helpers::int(1), &helpers::string(&arena, "1")), Ordering::Equal);
/// ```
pub fn compare_values<'a>(left: &DataValue<'a>, right: &DataValue<'a>) -> Ordering {
    match (left, right) {
        // Same types - use natural ordering
        (DataValue::Null, DataValue::Null) => Ordering::Equal,
        (DataValue::Bool(a), DataValue::Bool(b)) => a.cmp(b),
        (DataValue::Number(a), DataValue::Number(b)) => {
            let a_val = if let Number::Integer(a_val) = *a {
                a_val as f64
            } else if let Number::Float(a_val) = *a {
                a_val
            } else {
                0.0
            };
            let b_val = if let Number::Integer(b_val) = *b {
                b_val as f64
            } else if let Number::Float(b_val) = *b {
                b_val
            } else {
                0.0
            };
            a_val.partial_cmp(&b_val).unwrap_or(Ordering::Equal)
        }
        (DataValue::String(a), DataValue::String(b)) => a.cmp(b),
        (DataValue::Array(a), DataValue::Array(b)) => {
            // Compare arrays by comparing each element
            for (a_item, b_item) in a.iter().zip(b.iter()) {
                match compare_values(a_item, b_item) {
                    Ordering::Equal => continue,
                    other => return other,
                }
            }

            // If we get here, all common elements are equal
            a.len().cmp(&b.len())
        }

        // Different types - convert to comparable types
        (DataValue::Number(n), DataValue::String(s)) => {
            // Try to parse string as number
            match s.parse::<f64>() {
                Ok(s_val) => {
                    let n_val = if let Number::Integer(n_val) = *n {
                        n_val as f64
                    } else if let Number::Float(n_val) = *n {
                        n_val
                    } else {
                        0.0
                    };

                    n_val.partial_cmp(&s_val).unwrap_or(Ordering::Equal)
                }
                Err(_) => Ordering::Less, // Numbers come before strings in JSONLogic
            }
        }
        (DataValue::String(s), DataValue::Number(n)) => {
            // Reverse the comparison above
            match compare_values(&DataValue::Number(*n), &DataValue::String(s)) {
                Ordering::Less => Ordering::Greater,
                Ordering::Greater => Ordering::Less,
                Ordering::Equal => Ordering::Equal,
            }
        }
        (DataValue::Bool(b), other) => {
            // Convert boolean to number and compare
            let bool_as_num = if *b { 1 } else { 0 };
            compare_values(&DataValue::Number(Number::Integer(bool_as_num)), other)
        }
        (other, DataValue::Bool(b)) => {
            // Reverse the comparison above
            let bool_as_num = if *b { 1 } else { 0 };
            compare_values(other, &DataValue::Number(Number::Integer(bool_as_num)))
        }

        // Handle cases that don't have a natural ordering
        (DataValue::Null, _) => Ordering::Less,
        (_, DataValue::Null) => Ordering::Greater,

        // For all other combinations, compare by type order
        // (this is a fallback as all relevant cases should be handled above)
        _ => type_order(left).cmp(&type_order(right)),
    }
}

/// Determines if two DataValues are loosely equal according to JSONLogic rules.
///
/// Loose equality is similar to JavaScript's `==` operator.
/// - Different types may be considered equal after type conversion
/// - `null` is only equal to `null`
/// - `0` is equal to `"0"` (numeric string conversion)
///
/// # Examples
///
/// ```
/// use datavalue_rs::{helpers, Bump};
/// use datalogic_rs::value::loose_equals;
///
/// let arena = Bump::new();
///
/// assert!(loose_equals(&helpers::int(1), &helpers::string(&arena, "1")));
/// assert!(loose_equals(&helpers::boolean(true), &helpers::int(1)));
/// assert!(loose_equals(&helpers::boolean(false), &helpers::int(0)));
/// assert!(!loose_equals(&helpers::null(), &helpers::boolean(false)));
/// ```
pub fn loose_equals<'a>(left: &DataValue<'a>, right: &DataValue<'a>) -> bool {
    match (left, right) {
        // Number to string conversion
        (DataValue::Number(n), DataValue::String(s)) => {
            // Try to parse string as number
            match s.parse::<f64>() {
                Ok(s_val) => {
                    let n_val = if let Number::Integer(n_val) = *n {
                        n_val as f64
                    } else if let Number::Float(n_val) = *n {
                        n_val
                    } else {
                        0.0
                    };
                    (n_val - s_val).abs() < f64::EPSILON
                }
                Err(_) => false,
            }
        }
        (DataValue::String(_), DataValue::Number(_)) => loose_equals(right, left),

        // Boolean to number conversion
        (DataValue::Bool(b), DataValue::Number(n)) => {
            let b_val = if *b { 1.0 } else { 0.0 };
            let n_val = if let Number::Integer(n_val) = *n {
                n_val as f64
            } else if let Number::Float(n_val) = *n {
                n_val
            } else {
                0.0
            };
            (b_val - n_val).abs() < f64::EPSILON
        }
        (DataValue::Number(_), DataValue::Bool(_)) => loose_equals(right, left),

        // Boolean to string conversion
        (DataValue::Bool(b), DataValue::String(s)) => {
            // "true" == true, "false" == false
            match *s {
                "true" => *b,
                "false" => !*b,
                "1" => *b,
                "0" => !*b,
                _ => false,
            }
        }
        (DataValue::String(_), DataValue::Bool(_)) => loose_equals(right, left),

        // All other type combinations are not equal
        _ => left == right,
    }
}

/// Determines if two DataValues are strictly equal.
///
/// Strict equality is similar to JavaScript's `===` operator.
/// - Values must be of the same type to be considered equal
/// - Objects and arrays are compared by reference
///
/// # Examples
///
/// ```
/// use datavalue_rs::{helpers, Bump};
/// use datalogic_rs::value::strict_equals;
///
/// let arena = Bump::new();
///
/// assert!(strict_equals(&helpers::int(1), &helpers::int(1)));
/// assert!(!strict_equals(&helpers::int(1), &helpers::string(&arena, "1")));
/// assert!(!strict_equals(&helpers::boolean(true), &helpers::int(1)));
/// ```
pub fn strict_equals<'a>(left: &DataValue<'a>, right: &DataValue<'a>) -> bool {
    if left.get_type() != right.get_type() {
        return false;
    }

    left == right
}

// Helper function to assign a numeric order to value types
fn type_order<'a>(value: &DataValue<'a>) -> u8 {
    match value {
        DataValue::Null => 0,
        DataValue::Bool(_) => 1,
        DataValue::Number(_) => 2,
        DataValue::String(_) => 3,
        DataValue::Array(_) => 4,
        DataValue::Object(_) => 5,
        DataValue::DateTime(_) => 6,
        DataValue::Duration(_) => 7,
    }
}
