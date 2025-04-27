//! Compiler for the JSONLogic bytecode virtual machine.
//!
//! This module is responsible for compiling JSONLogic expressions into bytecode
//! that can be executed by the VM.

#![forbid(unsafe_code)]

use bumpalo::Bump;

use crate::{
    compiler::{
        lower::Lowering,
        types::{CompileError, Program},
    },
    core::ASTNode,
};

pub mod const_pool;
pub mod lower;
pub mod types;

/// Compile a parsed JSONLogic expression into bytecode.
///
/// This function takes an AST node and returns a compiled program that contains
/// bytecode instructions and a constant pool.
pub fn compile<'a>(node: &'a ASTNode<'a>, arena: &'a Bump) -> Result<Program<'a>, CompileError> {
    // Limit the instruction count to prevent infinite loops
    const MAX_INSTR_COUNT: usize = 10000;

    // Create a lowering handler
    let mut lowering = Lowering::new(arena);

    // Compile the AST
    lowering.compile(node)?;

    // Check if we exceeded the instruction limit
    let instr_count = lowering.instruction_count();
    if instr_count > MAX_INSTR_COUNT {
        return Err(CompileError::InstructionLimitExceeded(instr_count));
    }

    // Finalize and get the instructions and constant pool
    let (instructions, const_pool) = lowering.finalize();

    // Return a compiled program
    Ok(Program {
        instructions,
        const_pool,
    })
}
