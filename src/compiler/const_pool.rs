//! Constant pool management for the compiler.
//!
//! This module provides a ConstPool structure for managing constant values
//! used in JSONLogic expressions. It ensures each unique value is stored only once.

use crate::compiler::CompileError;
use bumpalo::Bump;
use datavalue_rs::{DataValue, Number};
use std::collections::HashMap;

/// A constant pool for storing literal values used in JSONLogic expressions.
///
/// The constant pool ensures each unique value is stored only once, reducing
/// memory usage and improving performance by avoiding duplicate storage.
pub struct ConstPool<'a> {
    // Maps values to their index in the pool
    value_to_index: HashMap<ValueKey<'a>, u32>,

    // The actual values stored in the pool
    values: Vec<DataValue<'a>>,

    // The arena for allocating values
    arena: &'a Bump,
}

/// Key type for HashMap lookups
///
/// We need a custom key type because DataValue can't implement Hash directly
/// due to containing floating point values and references.
#[derive(PartialEq, Eq, Hash)]
enum ValueKey<'a> {
    Null,
    Bool(bool),
    Int(i64),
    Float(u64), // Float bits as u64 for hashing
    String(&'a str),
    // We don't hash arrays or objects, since they're always allocated new
    // Likewise, we don't hash DateTime or Duration
}

impl<'a> ConstPool<'a> {
    /// Create a new constant pool using the provided arena.
    pub fn new(arena: &'a Bump) -> Self {
        Self {
            value_to_index: HashMap::new(),
            values: Vec::new(),
            arena,
        }
    }

    /// Add a value to the constant pool and return its index.
    ///
    /// If the value already exists in the pool, the existing index is returned.
    pub fn add(&mut self, value: &'a DataValue<'a>) -> Result<u32, CompileError> {
        // Check if we already have this value
        if let Some(key) = self.make_key(value) {
            if let Some(index) = self.value_to_index.get(&key) {
                return Ok(*index);
            }
        }

        // Value not found, add it to the pool
        let index = self.values.len() as u32;

        // Ensure the index fits in our immediate format (24 bits)
        if index >= 0x00FF_FFFF {
            return Err(CompileError::ConstPoolError(format!(
                "Constant pool index overflow: {}",
                index
            )));
        }

        // Store a clone of the value
        self.values.push(value.clone());

        // Also store the key for later lookups
        if let Some(key) = self.make_key(value) {
            self.value_to_index.insert(key, index);
        }

        Ok(index)
    }

    /// Add a new value to the constant pool.
    ///
    /// This is used for values that don't already exist in the arena.
    pub fn add_new(&mut self, value: DataValue<'a>) -> Result<u32, CompileError> {
        // Store in arena first
        let stored_value = self.arena.alloc(value);
        self.add(stored_value)
    }

    /// Convert a DataValue to a ValueKey for HashMap lookups.
    fn make_key(&self, value: &DataValue<'a>) -> Option<ValueKey<'a>> {
        match value {
            DataValue::Null => Some(ValueKey::Null),
            DataValue::Bool(b) => Some(ValueKey::Bool(*b)),
            DataValue::Number(Number::Integer(i)) => Some(ValueKey::Int(*i)),
            DataValue::Number(Number::Float(f)) => Some(ValueKey::Float(f.to_bits())),
            DataValue::String(s) => Some(ValueKey::String(s)),
            // Arrays and objects are not hashed as keys
            _ => None,
        }
    }

    /// Get a reference to the arena allocator
    pub fn arena(&self) -> &'a Bump {
        self.arena
    }

    /// Get a reference to the values in the constant pool
    pub fn values(&self) -> &[DataValue<'a>] {
        &self.values
    }

    /// Get the number of values in the constant pool.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if the constant pool is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Finalize the constant pool and return the values.
    pub fn finalize(self) -> Vec<DataValue<'a>> {
        self.values
    }
}
