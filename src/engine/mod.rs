//! Stack-based evaluation engine for JSONLogic expressions
//!
//! This module provides a non-recursive, stack-based approach for evaluating
//! JSONLogic expressions, which is more memory efficient for deeply nested expressions.

mod stack;

use crate::core::ASTNode;
use bumpalo::Bump;
use datavalue_rs::{DataValue, Result};

pub use stack::{Instruction, InstructionStack};

/// A precompiled JSONLogic rule containing precompiled instructions for efficient evaluation
#[derive(Debug)]
pub struct Logic<'a> {
    /// The precompiled instructions
    instruction_stack: &'a InstructionStack<'a>,
    /// The memory arena
    arena: &'a Bump,
}

impl<'a> Logic<'a> {
    /// Creates a new Logic by precompiling a token into an instruction stack
    pub fn new(token: &'a ASTNode<'a>, arena: &'a Bump) -> Result<Self> {
        // Create an instruction stack and compile the instructions
        let mut instruction_stack = InstructionStack::new(token);
        instruction_stack.compile()?;

        Ok(Self {
            instruction_stack: arena.alloc(instruction_stack),
            arena,
        })
    }

    /// Evaluates the precompiled logic against the provided data
    pub fn evaluate(&self, data: &'a DataValue<'a>) -> Result<&'a DataValue<'a>> {
        // Execute the precompiled instructions
        self.instruction_stack.evaluate(data, self.arena)
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
    token: &'a ASTNode<'a>,
    data: &'a DataValue<'a>,
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    let stack = InstructionStack::new(token);

    match stack.evaluate(data, arena) {
        Ok(result) => Ok(result),
        Err(e) => Err(e),
    }
}
