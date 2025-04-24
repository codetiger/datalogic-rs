//! JSONLogic implementation in Rust.
//!
//! This crate provides a fast, memory-efficient implementation of JSONLogic
//! using DataValue from the datavalue_rs crate.

pub mod engine;
pub mod operators;
pub mod optimizer;
pub mod parser;
pub mod value;

// Re-export the value extension trait for convenient usage
pub use value::DataValueExt;

// Re-export DataValue and helpers
pub use datavalue_rs::{helpers, Bump as DataBump, DataValue, Number};

// Re-export important parser types
pub use parser::{parser, OperatorType, ParserError, Token};

// Re-export optimizer functions
pub use optimizer::optimize;

// Re-export engine evaluation function
pub use engine::{evaluate, Logic};

use thiserror::Error;

/// Errors that can occur when using DataLogic functions
#[derive(Error, Debug)]
pub enum LogicError {
    #[error("Parser error: {0}")]
    ParserError(String),

    #[error("Evaluation error: {0}")]
    EvaluationError(String),

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Unknown operator: {operator}")]
    OperatorNotFoundError { operator: String },

    #[error("Invalid arguments: {0}")]
    InvalidArgumentsError(String),

    #[error("NaN detected in calculation")]
    NaNError,

    #[error("Error thrown: {type_name}")]
    ThrownError { type_name: String },
}

/// Main interface for JSONLogic operations
pub struct DataLogic {
    arena: DataBump,
}

impl Default for DataLogic {
    fn default() -> Self {
        Self::new()
    }
}

impl DataLogic {
    /// Creates a new DataLogic instance
    pub fn new() -> Self {
        Self {
            arena: DataBump::new(),
        }
    }

    /// Parses a logic rule from JSON string
    pub fn parse_logic(&self, rule_str: &str, format: Option<&str>) -> Result<Token, LogicError> {
        // Only supporting JSONLogic format for now
        if format.is_some() && format != Some("jsonlogic") {
            return Err(LogicError::ParserError(format!(
                "Unsupported format: {:?}",
                format
            )));
        }

        match parser::parser(rule_str, &self.arena) {
            Ok(token) => Ok((*token).clone()),
            Err(e) => Err(LogicError::ParserError(e.to_string())),
        }
    }

    /// Parses data from a JSON string
    pub fn parse_data(&self, data_str: &str) -> Result<DataValue, LogicError> {
        datavalue_rs::DataValue::from_str(&self.arena, data_str)
            .map_err(|e| LogicError::ParserError(format!("Failed to parse data: {}", e)))
    }

    /// Evaluates a logic rule against data
    pub fn evaluate<'a>(
        &'a self,
        logic: &'a Token<'a>,
        data: &'a DataValue<'a>,
    ) -> Result<DataValue<'a>, LogicError> {
        let result = evaluate(logic, data, &self.arena)
            .map_err(|e| LogicError::EvaluationError(e.to_string()))?;

        Ok(result.clone())
    }

    /// Compiles a JSONLogic rule into a precompiled Logic struct with instruction stack
    ///
    /// Note: The returned Logic instance is valid only for the lifetime of the DataLogic instance.
    pub fn compile(&self, rule_str: &str) -> Result<Logic<'_>, LogicError> {
        match parser::parser(rule_str, &self.arena) {
            Ok(token) => {
                // Compile the token into an instruction stack
                let compiled = Logic::new(token, &self.arena)
                    .map_err(|e| LogicError::ParserError(format!("Compilation error: {}", e)))?;

                Ok(compiled)
            }
            Err(e) => Err(LogicError::ParserError(e.to_string())),
        }
    }

    /// Evaluates a precompiled Logic against data
    pub fn apply_logic<'a>(
        &'a self,
        logic: &'a Logic<'a>,
        data: &'a DataValue<'a>,
    ) -> Result<DataValue<'a>, LogicError> {
        // Use the precompiled instruction stack for evaluation
        let result = logic
            .evaluate(data)
            .map_err(|e| LogicError::EvaluationError(e.to_string()))?;

        Ok(result.clone())
    }

    /// Parses, compiles, and evaluates a rule in one step
    pub fn evaluate_rule<'a>(
        &'a self,
        rule_str: &str,
        data: &'a DataValue<'a>,
    ) -> Result<DataValue<'a>, LogicError> {
        // Parse directly and evaluate without storing intermediate token
        match parser::parser(rule_str, &self.arena) {
            Ok(token) => {
                // Instead of creating a Logic struct, create the CompiledLogic directly
                // and use it for evaluation
                let compiled = Logic::new(token, &self.arena)
                    .map_err(|e| LogicError::ParserError(format!("Compilation error: {}", e)))?;

                let result = compiled
                    .evaluate(data)
                    .map_err(|e| LogicError::EvaluationError(e.to_string()))?;

                Ok(result.clone())
            }
            Err(e) => Err(LogicError::ParserError(e.to_string())),
        }
    }

    /// Reset the arena to free memory
    pub fn reset(&mut self) {
        self.arena.reset();
    }
}

/// Extension to DataValue for test comparison
pub trait DataValueTestExt<'a> {
    /// Compares two DataValues for equality according to JSONLogic rules
    fn equals(&self, other: &DataValue<'a>) -> bool;
}

impl<'a> DataValueTestExt<'a> for DataValue<'a> {
    fn equals(&self, other: &DataValue<'a>) -> bool {
        use value::loose_equals;
        loose_equals(self, other)
    }
}
