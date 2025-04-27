//! JSONLogic implementation in Rust.
//!
//! This crate provides a fast, memory-efficient implementation of JSONLogic
//! using DataValue from the datavalue_rs crate.

pub mod compiler;
pub mod core;
pub mod value;
pub mod vm_stack;

use bumpalo::Bump;
// Re-export the value extension trait for convenient usage
pub use value::DataValueExt;

// Re-export DataValue and helpers
pub use datavalue_rs::{helpers, DataValue, Number};

// Re-export important parser types
pub use core::{parser, ASTNode, OperatorType, ParserError};

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
    arena: Bump,
}

impl Default for DataLogic {
    fn default() -> Self {
        Self::new()
    }
}

impl DataLogic {
    /// Creates a new DataLogic instance
    pub fn new() -> Self {
        Self { arena: Bump::new() }
    }

    /// Parses a logic rule from JSON string
    pub fn parse_logic(&self, rule_str: &str, format: Option<&str>) -> Result<ASTNode, LogicError> {
        // Only supporting JSONLogic format for now
        if format.is_some() && format != Some("jsonlogic") {
            return Err(LogicError::ParserError(format!(
                "Unsupported format: {:?}",
                format
            )));
        }

        match core::parser(rule_str, &self.arena) {
            Ok(token) => Ok((*token).clone()),
            Err(e) => Err(LogicError::ParserError(e.to_string())),
        }
    }

    /// Parses data from a JSON string
    pub fn parse_data(&self, data_str: &str) -> Result<DataValue, LogicError> {
        datavalue_rs::DataValue::from_str(&self.arena, data_str)
            .map_err(|e| LogicError::ParserError(format!("Failed to parse data: {}", e)))
    }

    /// Demonstrates the VM execution by running simple arithmetic rules
    /// This is a simplified version for demonstration purposes
    pub fn demo_vm_arithmetic(&self) -> Result<i64, LogicError> {
        // Create a simple rule: {"+":[3,4]}
        let rule_str = r#"{"+":[3,4]}"#;
        let ast = self.parse_logic(rule_str, None)?;

        // Directly compile the AST
        let program = compiler::compile(&ast, &self.arena)
            .map_err(|e| LogicError::EvaluationError(format!("Compilation failed: {}", e)))?;

        // Create an empty data context
        let data = self.arena.alloc(DataValue::Null);
        let context = vm_stack::DataContext::new(data);

        // Run the program and return the result
        let result = vm_stack::run(&program, &context, &self.arena);

        // Convert the result to a simple i64
        match result {
            DataValue::Number(Number::Integer(i)) => Ok(i),
            _ => Err(LogicError::EvaluationError(
                "Expected integer result".to_string(),
            )),
        }
    }
}
