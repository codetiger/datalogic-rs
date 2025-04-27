//! Truthy evaluation for JSONLogic.
//!
//! This module implements the truthy behavior for DataValue
//! according to JSONLogic rules.

use datavalue_rs::{DataValue, Number};

/// Determines if a value is truthy according to JSONLogic rules.
///
/// In JSONLogic:
/// - `false`, `null`, `""` (empty string), `0`, `[]` (empty array) are falsy
/// - Everything else is truthy
///
/// This follows JavaScript's concept of truthiness with one exception:
/// - In JavaScript, empty arrays (`[]`) are truthy
/// - In JSONLogic, empty arrays are falsy
///
/// # Examples
///
/// ```
/// use datavalue_rs::{helpers, Bump};
/// use datalogic_rs::value::is_truthy;
///
/// let arena = Bump::new();
///
/// assert!(!is_truthy(&helpers::null()));
/// assert!(!is_truthy(&helpers::boolean(false)));
/// assert!(!is_truthy(&helpers::int(0)));
/// assert!(!is_truthy(&helpers::string(&arena, "")));
/// assert!(!is_truthy(&helpers::array(&arena, vec![])));
///
/// assert!(is_truthy(&helpers::boolean(true)));
/// assert!(is_truthy(&helpers::int(1)));
/// assert!(is_truthy(&helpers::int(-1)));
/// assert!(is_truthy(&helpers::float(0.1)));
/// assert!(is_truthy(&helpers::string(&arena, "hello")));
/// assert!(is_truthy(&helpers::string(&arena, "0")));
/// assert!(is_truthy(&helpers::string(&arena, "false")));
/// ```
pub fn is_truthy<'a>(value: &DataValue<'a>) -> bool {
    match *value {
        // Explicit falsy values
        DataValue::Null => false,
        DataValue::Bool(b) => b,

        // Numbers - only 0 and NaN are falsy
        DataValue::Number(Number::Integer(i)) if i != 0 => true,
        DataValue::Number(Number::Float(f)) if f != 0.0 && !f.is_nan() => true,
        DataValue::Number(_) => false,

        // String - only empty string is falsy
        DataValue::String(s) => !s.is_empty(),

        // Array - only empty array is falsy (different from JavaScript)
        DataValue::Array(items) => !items.is_empty(),

        // Object - always truthy
        DataValue::Object(_) => true,

        // DateTime - always truthy
        DataValue::DateTime(_) => true,

        // Duration - false if zero
        DataValue::Duration(d) => !d.is_zero(),
    }
}
