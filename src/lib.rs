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
pub use datavalue_rs::{helpers, Bump, DataValue, Number};

// Re-export important parser types
pub use parser::{parser, OperatorType, ParserError, Token};

// Re-export optimizer functions
pub use optimizer::optimize;

// Re-export engine evaluation function
pub use engine::evaluate;
