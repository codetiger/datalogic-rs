//! Value extensions for JSONLogic.
//!
//! This module extends the DataValue type from the datavalue-rs crate
//! with functionalities needed for JSONLogic evaluation.

mod compare;
mod convert;
#[cfg(test)]
mod tests;
mod truthy;

pub use compare::*;
pub use convert::*;
pub use truthy::*;

use datavalue_rs::DataValue;

// Re-export the datavalue-rs crate so users can access it directly
pub use datavalue_rs;

/// Extension trait for DataValue to support JSONLogic operations
pub trait DataValueExt<'a> {
    /// Evaluates whether a value is truthy according to JSONLogic rules
    fn is_truthy(&self) -> bool;

    /// Compare two DataValues
    fn compare(&self, other: &DataValue<'a>) -> std::cmp::Ordering;

    /// Equal (loose equality, similar to JavaScript ==)
    fn loose_equals(&self, other: &DataValue<'a>) -> bool;

    /// Strict equal (strict equality, similar to JavaScript ===)
    fn strict_equals(&self, other: &DataValue<'a>) -> bool;

    /// Coerce to number according to JSONLogic rules
    fn coerce_to_number(&self) -> DataValue<'a>;

    /// Convert to number according to JSONLogic rules
    fn convert_to_number(&self) -> DataValue<'a>;

    /// Modulo operation
    fn modulo(&self, other: &DataValue<'a>) -> DataValue<'a>;

    /// Check if key exists in DataValue
    fn key_exists(&self, key: &str) -> bool;
}

// Implement the extension trait for DataValue
impl<'a> DataValueExt<'a> for DataValue<'a> {
    fn is_truthy(&self) -> bool {
        truthy::is_truthy(self)
    }

    fn compare(&self, other: &DataValue<'a>) -> std::cmp::Ordering {
        compare::compare_values(self, other)
    }

    fn loose_equals(&self, other: &DataValue<'a>) -> bool {
        compare::loose_equals(self, other)
    }

    fn strict_equals(&self, other: &DataValue<'a>) -> bool {
        compare::strict_equals(self, other)
    }

    fn coerce_to_number(&self) -> DataValue<'a> {
        convert::coerce_to_number(self)
    }

    fn convert_to_number(&self) -> DataValue<'a> {
        convert::convert_to_number(self)
    }

    fn modulo(&self, other: &DataValue<'a>) -> DataValue<'a> {
        convert::modulo(self, other)
    }

    fn key_exists(&self, key: &str) -> bool {
        if key.is_empty() {
            return false;
        }

        if key.contains('.') {
            let key_components = key.split('.').collect::<Vec<&str>>();
            let mut current = Some(self);

            for component in key_components {
                match current {
                    Some(DataValue::Object(_)) => {
                        current = current.unwrap().get(component);
                    }
                    _ => return false,
                }
            }

            current.is_some()
        } else {
            match self {
                DataValue::Object(_) => self.contains_key(key),
                _ => false,
            }
        }
    }
}
