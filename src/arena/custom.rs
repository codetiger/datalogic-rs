use crate::arena::DataArena;
use crate::logic::Result;
use crate::value::DataValue;
use crate::LogicError;
use std::collections::HashMap;
use std::fmt;

/// Trait for custom JSONLogic operators
pub trait CustomOperator: fmt::Debug + Send + Sync {
    /// Evaluate the custom operator with the given arguments
    ///
    /// This function takes owned DataValue arguments and returns an owned DataValue.
    /// The actual allocation in the arena is handled internally.
    fn evaluate<'a>(
        &self,
        args: &'a [DataValue<'a>],
        arena: &'a DataArena,
    ) -> Result<&'a DataValue<'a>>;
}

/// Registry for custom operator functions
#[derive(Default)]
pub struct CustomOperatorRegistry {
    operators: HashMap<String, Box<dyn CustomOperator>>,
}

impl CustomOperatorRegistry {
    /// Creates a new empty custom operator registry
    pub fn new() -> Self {
        Self {
            operators: HashMap::new(),
        }
    }

    /// Registers a custom operator function
    pub fn register(&mut self, name: &str, operator: Box<dyn CustomOperator>) {
        self.operators.insert(name.to_string(), operator);
    }

    /// Returns a reference to a custom operator by name
    pub fn get(&self, name: &str) -> Option<&dyn CustomOperator> {
        self.operators.get(name).map(|op| op.as_ref())
    }
}

/// A function type that works with owned DataValues rather than arena references
///
/// This type allows implementing custom operators without dealing with
/// arena allocation or lifetimes. The function takes owned DataValues
/// and returns an owned DataValue.
pub type SimpleOperatorFn = fn(Vec<DataValue>) -> std::result::Result<DataValue, String>;

/// An adapter that converts between the simple owned-value API and the arena-based API
///
/// This adapter wraps a function that works with owned DataValues and implements
/// the CustomOperator trait, handling all arena allocation details internally.
#[derive(Debug)]
pub struct SimpleOperatorAdapter {
    function: SimpleOperatorFn,
    name: String,
}

impl SimpleOperatorAdapter {
    /// Create a new adapter wrapping a simple operator function
    pub fn new(name: &str, function: SimpleOperatorFn) -> Self {
        Self {
            function,
            name: name.to_string(),
        }
    }
}

impl CustomOperator for SimpleOperatorAdapter {
    fn evaluate<'a>(
        &self,
        args: &'a [DataValue<'a>],
        arena: &'a DataArena,
    ) -> Result<&'a DataValue<'a>> {
        // Convert arena-referenced DataValues to owned DataValues
        let owned_args = args.iter().map(|arg| arg.to_owned()).collect::<Vec<_>>();

        // Call the user's simple function that works with owned values
        match (self.function)(owned_args) {
            Ok(result) => {
                // Handle basic scalar types directly
                match result {
                    DataValue::Null => Ok(arena.null_value()),
                    DataValue::Bool(true) => Ok(arena.true_value()),
                    DataValue::Bool(false) => Ok(arena.false_value()),
                    DataValue::Number(n) => Ok(arena.alloc(DataValue::Number(n))),
                    DataValue::String(s) => {
                        let s_arena = arena.alloc_str(s);
                        Ok(arena.alloc(DataValue::String(s_arena)))
                    }
                    // For complex types like Array and Object, convert to string as a fallback
                    DataValue::Array(_) | DataValue::Object(_) => {
                        let str_rep = format!("{:?}", result);
                        let s_arena = arena.alloc_str(&str_rep);
                        Ok(arena.alloc(DataValue::String(s_arena)))
                    }
                    // Handle DateTime and Duration types
                    DataValue::DateTime(dt) => Ok(arena.alloc(DataValue::DateTime(dt))),
                    DataValue::Duration(dur) => Ok(arena.alloc(DataValue::Duration(dur))),
                }
            }
            Err(msg) => Err(LogicError::ParseError {
                reason: format!("Error in operator '{}': {}", self.name, msg),
            }),
        }
    }
}
