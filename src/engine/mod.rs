//! Stack-based evaluation engine for JSONLogic expressions
//!
//! This module provides a non-recursive, stack-based approach for evaluating
//! JSONLogic expressions, which is more memory efficient for deeply nested expressions.

mod stack;

use crate::parser::Token;
use bumpalo::Bump;
use datavalue_rs::{DataValue, Result};

pub use stack::{Instruction, InstructionStack};

/// A precompiled JSONLogic rule containing precompiled instructions for efficient evaluation
#[derive(Debug)]
pub struct CompiledLogic<'a> {
    /// The precompiled instructions
    instructions: Vec<Instruction<'a>>,
    /// The memory arena
    arena: &'a Bump,
}

impl<'a> CompiledLogic<'a> {
    /// Creates a new CompiledLogic by precompiling a token into an instruction stack
    pub fn new(token: &'a Token<'a>, arena: &'a Bump) -> Result<Self> {
        // Create an instruction stack and compile the instructions
        let mut stack = InstructionStack::new(token);
        stack.compile(token)?;

        // Store the compiled instructions
        let instructions = stack.instructions.clone();

        Ok(Self {
            instructions,
            arena,
        })
    }

    /// Evaluates the precompiled logic against the provided data
    pub fn apply(&self, data: &'a DataValue<'a>) -> Result<&'a DataValue<'a>> {
        // Create a new stack with our precompiled instructions
        let mut stack = InstructionStack {
            instructions: self.instructions.clone(),
        };

        // Execute the precompiled instructions
        stack.evaluate(data, self.arena)
    }
}

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
