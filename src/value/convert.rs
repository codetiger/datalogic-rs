//! Value conversion functions for JSONLogic
//!
//! This module provides functions for coercing DataValue values according to
//! JSONLogic type conversion rules.

use datavalue_rs::{helpers, DataValue, Number};

/// Coerces a value to a number according to JSONLogic rules
///
/// # Rules
/// - Numbers remain as numbers
/// - Empty strings become 0
/// - Strings with numeric content are converted to numbers
/// - Non-numeric strings become 0
/// - Booleans: true → 1, false → 0
/// - Arrays:
///   - Empty arrays become 0
///   - Single-element arrays are coerced as their single element
///   - Multi-element arrays become 0
/// - Objects become 0
/// - Null becomes 0
///
/// # Arguments
///
/// * `value` - The value to coerce
///
/// # Returns
///
/// A DataValue containing the coerced number
pub fn coerce_to_number<'a>(value: &DataValue<'a>) -> DataValue<'a> {
    match value {
        // Numbers are already numbers, just clone them
        DataValue::Number(num) => match num {
            Number::Integer(i) => helpers::int(*i),
            Number::Float(f) => helpers::float(*f),
        },

        // Booleans: true → 1, false → 0
        DataValue::Bool(b) => {
            if *b {
                helpers::int(1)
            } else {
                helpers::int(0)
            }
        }

        // Strings: parse numeric content, empty or non-numeric become 0
        DataValue::String(s) => {
            if s.is_empty() {
                return helpers::int(0);
            }

            // Try to parse as number
            if let Ok(i) = s.parse::<i64>() {
                helpers::int(i)
            } else if let Ok(f) = s.parse::<f64>() {
                helpers::float(f)
            } else {
                helpers::int(0)
            }
        }

        // Arrays: empty → 0, single element → coerce that element, multiple elements → 0
        DataValue::Array(arr) => {
            if arr.is_empty() {
                helpers::int(0)
            } else if arr.len() == 1 {
                coerce_to_number(&arr[0])
            } else {
                helpers::int(0)
            }
        }

        // Null, objects, and other types become 0
        DataValue::Null
        | DataValue::Object(_)
        | DataValue::DateTime(_)
        | DataValue::Duration(_) => helpers::int(0),
    }
}

pub fn convert_to_number<'a>(value: &DataValue<'a>) -> DataValue<'a> {
    match *value {
        DataValue::Number(num) => match num {
            Number::Integer(i) => helpers::int(i),
            Number::Float(f) => {
                if f.fract() == 0.0 {
                    helpers::int(f as i64)
                } else {
                    helpers::float(f)
                }
            },
        },
        _ => coerce_to_number(value),
    }
}

pub fn modulo<'a>(value: &DataValue<'a>, other: &DataValue<'a>) -> DataValue<'a> {
    match (value, other) {
        (DataValue::Number(num1), DataValue::Number(num2)) => {
            if let (Number::Integer(i1), Number::Integer(i2)) = (num1, num2) {
                helpers::int(i1 % i2)
            } else {
                helpers::int(0)
            }
        }
        _ => helpers::int(0),
    }
}


#[cfg(test)]
mod tests {
    use bumpalo::Bump;

    use super::*;

    #[test]
    fn test_coerce_to_number() {
        // Numbers should remain unchanged
        assert_eq!(coerce_to_number(&helpers::int(42)), helpers::int(42));
        assert_eq!(
            coerce_to_number(&helpers::float(3.14)),
            helpers::float(3.14)
        );

        // Booleans
        assert_eq!(coerce_to_number(&helpers::boolean(true)), helpers::int(1));
        assert_eq!(coerce_to_number(&helpers::boolean(false)), helpers::int(0));

        // Strings
        let arena = Bump::new();
        let str_val = helpers::string(&arena, "42");
        assert_eq!(coerce_to_number(&str_val), helpers::int(42));

        let str_val = helpers::string(&arena, "3.14");
        assert_eq!(coerce_to_number(&str_val), helpers::float(3.14));

        let str_val = helpers::string(&arena, "");
        assert_eq!(coerce_to_number(&str_val), helpers::int(0));

        let str_val = helpers::string(&arena, "abc");
        assert_eq!(coerce_to_number(&str_val), helpers::int(0));

        // Arrays
        let empty_array = DataValue::Array(&[]);
        assert_eq!(coerce_to_number(&empty_array), helpers::int(0));

        // Create a long-lived array for testing
        let single_value = helpers::int(42);
        let single_array_values = [single_value];
        let single_array = DataValue::Array(&single_array_values);
        assert_eq!(coerce_to_number(&single_array), helpers::int(42));

        // Create a long-lived array for testing
        let val1 = helpers::int(1);
        let val2 = helpers::int(2);
        let multi_array_values = [val1, val2];
        let multi_array = DataValue::Array(&multi_array_values);
        assert_eq!(coerce_to_number(&multi_array), helpers::int(0));

        // Null
        assert_eq!(coerce_to_number(&DataValue::Null), helpers::int(0));

        // Object
        let object = DataValue::Object(&[]);
        assert_eq!(coerce_to_number(&object), helpers::int(0));
    }
}
