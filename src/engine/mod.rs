//! Stack-based evaluation engine for JSONLogic expressions
//!
//! This module provides a non-recursive, stack-based approach for evaluating
//! JSONLogic expressions, which is more memory efficient for deeply nested expressions.

mod stack;

use crate::parser::Token;
use bumpalo::Bump;
use datavalue_rs::{DataValue, Result};

pub use stack::InstructionStack;

/// Evaluates a JSONLogic expression using a stack-based approach
///
/// # Arguments
///
/// * `token` - The token to evaluate
/// * `data` - The data context for variable resolution
/// * `arena` - The memory arena for allocations
///
/// # Returns
///
/// A DataValue containing the result of the evaluation
pub fn evaluate<'a>(
    token: &'a Token<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let mut stack = InstructionStack::new(token);

    // For now, we're implementing a simplified version that only handles
    // addition as a proof of concept
    match stack.evaluate(data, arena) {
        Ok(result) => Ok(result),
        Err(e) => Err(e),
    }
}
