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
}
