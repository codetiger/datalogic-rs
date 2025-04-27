//! Converts the optimized AST into bytecode.
//!
//! This module is responsible for the final stage of compilation:
//! converting the optimized AST into bytecode instructions.
//! It implements the Lowering struct to perform this lowering.

use bumpalo::Bump;

use crate::{
    compiler::{
        const_pool::ConstPool,
        types::{CompileError, Instr, OpCode, OpTag},
    },
    core::{ASTNode, OperatorType},
};
use datavalue_rs::DataValue;

use super::types::CallTag;

/// Lowering implementation for transforming ASTNode to bytecode
pub struct Lowering<'a> {
    /// Instructions generated during lowering
    instructions: Vec<Instr>,

    /// Constant pool for storing literal values
    const_pool: ConstPool<'a>,

    /// Current number of local variables
    locals_count: u32,

    /// Maximum number of local variables
    locals_limit: u32,

    /// Reference to the arena
    arena: &'a Bump,

    /// Flag to track if a return has been emitted at the end
    return_emitted: bool,
}

impl<'a> Lowering<'a> {
    /// Create a new lowering handler with an arena
    pub fn new(arena: &'a Bump) -> Self {
        Self {
            instructions: Vec::new(),
            const_pool: ConstPool::new(arena),
            locals_count: 0,
            locals_limit: 256, // Default to 256 locals
            arena,
            return_emitted: false,
        }
    }

    /// Get access to the constant pool
    pub fn const_pool(&mut self) -> &mut ConstPool<'a> {
        &mut self.const_pool
    }

    /// Compile an ASTNode to bytecode instructions
    pub fn compile(&mut self, node: &'a ASTNode<'a>) -> Result<(), CompileError> {
        // Reset the return flag at the start of compilation
        self.return_emitted = false;

        // Process the node based on its type
        match node {
            ASTNode::Literal(value) => self.compile_literal(value)?,
            ASTNode::Variable { path, default, scope_jump } => {
                self.compile_variable(path, default, scope_jump)?
            }
            ASTNode::DynamicVariable { path_expr, default, scope_jump } => {
                self.compile_dynamic_variable(path_expr, default, scope_jump)?
            }
            ASTNode::Operator { op_type, args } => {
                // Handle the operator based on its type - this would call appropriate op compiler modules
                self.compile_operator(*op_type, args)?;
            }
            ASTNode::Array(items) => {
                // Compile each array item
                for item in items {
                    self.compile_without_return(item)?;
                }

                // Emit a MakeArray instruction with the array size
                self.emit(Instr::new(OpCode::Call, 0));
            }
            ASTNode::ArrayLiteral(items) => {
                let data = self.arena.alloc(DataValue::Array(items));
                let const_idx = self.const_pool.add(data)?;
                self.emit(Instr::new(OpCode::LoadConst, const_idx));
            }
            _ => {
                return Err(CompileError::LoweringError(format!(
                    "Node type not implemented: {:?}",
                    node
                )));
            }
        }

        // Emit the Return instruction as the last instruction if not already emitted
        self.ensure_return();

        Ok(())
    }

    /// Compile a node without adding a return instruction
    fn compile_without_return(&mut self, node: &'a ASTNode<'a>) -> Result<(), CompileError> {
        // Save the current return flag
        let old_return_emitted = self.return_emitted;

        // Set the flag to true to prevent return emission
        self.return_emitted = true;

        // Compile the node
        match node {
            ASTNode::Literal(value) => self.compile_literal(value)?,
            ASTNode::Variable { path, default, scope_jump } => {
                self.compile_variable(path, default, scope_jump)?
            }
            ASTNode::DynamicVariable { path_expr, default, scope_jump } => {
                self.compile_dynamic_variable(path_expr, default, scope_jump)?
            }
            ASTNode::Operator { op_type, args } => {
                self.compile_operator(*op_type, args)?;
            }
            ASTNode::Array(items) => {
                // Compile each array item
                for item in items {
                    self.compile_without_return(item)?;
                }

                // Emit a MakeArray instruction
                self.emit(Instr::new(OpCode::Call, 0));
            }
            ASTNode::ArrayLiteral(items) => {
                // For array literals, add the entire array to the constant pool
                let data = self.arena.alloc(DataValue::Array(items));
                let const_idx = self.const_pool.add(data)?;
                self.emit(Instr::new(OpCode::LoadConst, const_idx));
            }
            _ => {
                return Err(CompileError::LoweringError(format!(
                    "Node type not implemented: {:?}",
                    node
                )));
            }
        }

        // Restore the previous return flag
        self.return_emitted = old_return_emitted;

        Ok(())
    }

    // Implementation for variable compilation
    fn compile_variable(
        &mut self,
        path: &'a DataValue<'a>,
        default: &Option<&'a DataValue<'a>>,
        scope_jump: &Option<usize>,
    ) -> Result<(), CompileError> {
        // Add the path to the constant pool
        let path_idx = self.const_pool.add(path)?;

        // Handle default value if provided
        let _default_idx = if let Some(default_value) = default {
            // Add default value to const pool
            self.const_pool.add(default_value)?
        } else {
            // Add null to represent no default
            let null_val = self.arena.alloc(DataValue::Null);
            self.const_pool.add(null_val)?
        };

        // Handle scope jump if provided
        let _scope_jump_idx = if let Some(jump) = scope_jump {
            // Add scope jump as an integer to const pool
            let jump_val = self.arena.alloc(DataValue::Number(
                datavalue_rs::Number::Integer(*jump as i64)
            ));
            self.const_pool.add(jump_val)?
        } else {
            // Add -1 to represent no scope jump
            let no_jump_val = self.arena.alloc(DataValue::Number(
                datavalue_rs::Number::Integer(-1)
            ));
            self.const_pool.add(no_jump_val)?
        };

        // Emit the LoadVar instruction with the path index
        self.emit(Instr::new(OpCode::LoadVar, path_idx));

        Ok(())
    }

    // Implementation for dynamic variable compilation (where path is computed at runtime)
    fn compile_dynamic_variable(
        &mut self,
        path_expr: &'a Box<ASTNode<'a>>,
        default: &'a Option<Box<ASTNode<'a>>>,
        scope_jump: &'a Option<usize>,
    ) -> Result<(), CompileError> {
        // First compile the path expression that will give us the variable path
        self.compile_without_return(&**path_expr)?;
        
        // After compiling the path expression, the result will be on the stack
        
        // Handle default value if provided
        if let Some(default_expr) = default {
            // Compile the default value expression
            self.compile_without_return(&**default_expr)?;
        } else {
            // Push null as the default value
            let null_val = self.arena.alloc(DataValue::Null);
            let null_idx = self.const_pool.add(null_val)?;
            self.emit(Instr::new(OpCode::LoadConst, null_idx));
        }
        
        // Handle scope jump if provided
        let scope_jump_val = if let Some(jump) = scope_jump {
            // Add scope jump as an integer to const pool
            self.arena.alloc(DataValue::Number(
                datavalue_rs::Number::Integer(*jump as i64)
            ))
        } else {
            // Add -1 to represent no scope jump
            self.arena.alloc(DataValue::Number(
                datavalue_rs::Number::Integer(-1)
            ))
        };
        let scope_jump_idx = self.const_pool.add(scope_jump_val)?;
        self.emit(Instr::new(OpCode::LoadConst, scope_jump_idx));
        
        // Emit the LoadDynamicVar instruction to load the variable using the path on the stack
        self.emit(Instr::new(OpCode::LoadDynamicVar, 0));

        Ok(())
    }

    // Implementation for operator compilation
    fn compile_operator(
        &mut self,
        op_type: OperatorType,
        args: &'a ASTNode<'a>,
    ) -> Result<(), CompileError> {
        // First compile arguments based on operator type
        match op_type {
            // Arithmetic operators
            OperatorType::Add
            | OperatorType::Subtract
            | OperatorType::Multiply
            | OperatorType::Divide
            | OperatorType::Modulo
            | OperatorType::Min
            | OperatorType::Max => {
                self.compile_arithmetic_args(op_type, args)?;
            }

            // Comparison operators
            OperatorType::Equal
            | OperatorType::NotEqual
            | OperatorType::LT
            | OperatorType::LTE
            | OperatorType::GT
            | OperatorType::GTE
            | OperatorType::StrictEqual
            | OperatorType::StrictNotEqual
            | OperatorType::In => {
                self.compile_comparison_args(op_type, args)?;
            }

            // Logical operators
            OperatorType::And | OperatorType::Or | OperatorType::Not | OperatorType::DoubleBang => {
                self.compile_logical_args(op_type, args)?;
            }

            // If operator requires special handling
            OperatorType::If => {
                return self.compile_if_args(args);
            }

            // Variable access
            OperatorType::Var => {
                return self.compile_var_args(args);
            }
            
            // Array/string operations
            OperatorType::Merge => {
                return self.compile_merge_args(args);
            }
            
            OperatorType::Cat => {
                return self.compile_cat_args(args);
            }
            
            OperatorType::Substring => {
                return self.compile_substr_args(args);
            }

            // Missing variables check operations
            OperatorType::Missing => {
                return self.compile_missing_args(args);
            }
            
            OperatorType::MissingSome => {
                return self.compile_missing_some_args(args);
            }

            // Not implemented yet
            _ => {
                return Err(CompileError::LoweringError(format!(
                    "Operator type not implemented: {:?}",
                    op_type
                )));
            }
        }

        Ok(())
    }

    // Helper method for compiling arithmetic operator arguments
    fn compile_arithmetic_args(
        &mut self,
        op_type: OperatorType,
        args: &'a ASTNode<'a>,
    ) -> Result<(), CompileError> {
        if let ASTNode::Array(items) = args {
            // Compile each item in the array, but in reverse order for the stack
            let arg_count = items.len() as u32;
            
            // Push items in reverse order so they'll be popped in correct order
            for item in items.iter().rev() {
                self.compile_without_return(item)?;
            }

            // Map operator type to binary tag and emit variadic instruction
            match op_type {
                OperatorType::Add => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Add as u32) << 16 | arg_count,
                )),
                OperatorType::Subtract => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Sub as u32) << 16 | arg_count,
                )),
                OperatorType::Multiply => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Mul as u32) << 16 | arg_count,
                )),
                OperatorType::Divide => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Div as u32) << 16 | arg_count,
                )),
                OperatorType::Modulo => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Mod as u32) << 16 | arg_count,
                )),
                OperatorType::Min => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Min as u32) << 16 | arg_count,
                )),
                OperatorType::Max => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Max as u32) << 16 | arg_count,
                )),
                _ => {
                    return Err(CompileError::LoweringError(
                        "Unknown arithmetic operator".into(),
                    ))
                }
            }
        } else if let ASTNode::ArrayLiteral(items) = args {
            // Compile each item in the array, but in reverse order for the stack
            let arg_count = items.len() as u32;
            
            // Add items to constant pool in reverse order
            for item in items.iter().rev() {
                let const_idx = self.const_pool.add(item)?;
                self.emit(Instr::new(OpCode::LoadConst, const_idx));
            }

            // Map operator type to binary tag and emit variadic instruction
            match op_type {
                OperatorType::Add => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Add as u32) << 16 | arg_count,
                )),
                OperatorType::Subtract => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Sub as u32) << 16 | arg_count,
                )),
                OperatorType::Multiply => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Mul as u32) << 16 | arg_count,
                )),
                OperatorType::Divide => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Div as u32) << 16 | arg_count,
                )),
                OperatorType::Modulo => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Mod as u32) << 16 | arg_count,
                )),
                OperatorType::Min => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Min as u32) << 16 | arg_count,
                )),
                OperatorType::Max => self.emit(Instr::new(
                    OpCode::Variadic,
                    (OpTag::Max as u32) << 16 | arg_count,
                )),
                _ => {
                    return Err(CompileError::LoweringError(
                        "Unknown arithmetic operator".into(),
                    ))
                }
            }
        } else if let ASTNode::Literal(value) = args {
            let const_idx = self.const_pool.add(value)?;
            self.emit(Instr::new(OpCode::LoadConst, const_idx));
            self.emit(Instr::new(OpCode::Variadic, (OpTag::Add as u32) << 16 | 1));
        } else {
            return Err(CompileError::LoweringError(format!(
                "Expected array of arguments, got: {:?}",
                args
            )));
        }

        Ok(())
    }

    // Helper method for compiling comparison operator arguments
    fn compile_comparison_args(
        &mut self,
        op_type: OperatorType,
        args: &'a ASTNode<'a>,
    ) -> Result<(), CompileError> {
        let op_tag = match op_type {
            OperatorType::Equal => OpTag::Equal,
            OperatorType::StrictEqual => OpTag::StrictEqual,
            OperatorType::NotEqual => OpTag::NotEqual,
            OperatorType::StrictNotEqual => OpTag::StrictNotEqual,
            OperatorType::LT => OpTag::LessThan,
            OperatorType::LTE => OpTag::LessThanOrEqual,
            OperatorType::GT => OpTag::GreaterThan,
            OperatorType::GTE => OpTag::GreaterThanOrEqual,
            OperatorType::In => OpTag::In,
            _ => {
                return Err(CompileError::LoweringError(
                    "Unsupported comparison operator".into(),
                ));
            }
        };

        match args {
            ASTNode::Array(items) => {
                // Compile the arguments in reverse order
                for item in items.iter().rev() {
                    self.compile_without_return(item)?;
                }
    
                // Use Variadic opcode with argument count = items.len()
                self.emit(Instr::new(OpCode::Variadic, (op_tag as u32) << 16 | items.len() as u32));
            }
            ASTNode::ArrayLiteral(items) => {
                let data = self.arena.alloc(DataValue::Array(items));
                let const_idx = self.const_pool.add(data)?;
                self.emit(Instr::new(OpCode::LoadConst, const_idx));
                self.emit(Instr::new(OpCode::Variadic, (op_tag as u32) << 16 | 1));
            }
            _ => {
                return Err(CompileError::LoweringError(format!(
                    "Expected array of arguments, got: {:?}",
                    args
                )));
            }
        }

        Ok(())
    }

    // Helper method for compiling logical operator arguments
    fn compile_logical_args(
        &mut self,
        op_type: OperatorType,
        args: &'a ASTNode<'a>,
    ) -> Result<(), CompileError> {
        match args {
            ASTNode::Literal(value) => {
                let const_idx = self.const_pool.add(value)?;
                self.emit(Instr::new(OpCode::LoadConst, const_idx));
                self.emit(Instr::new(
                    OpCode::Variadic,
                    (match op_type {
                        OperatorType::And => OpTag::And,
                        OperatorType::Or => OpTag::Or,
                        OperatorType::Not => OpTag::Not,
                        OperatorType::DoubleBang => OpTag::DNot,
                        _ => return Err(CompileError::LoweringError("Unknown logical operator".into())),
                    } as u32) << 16 | 1,
                ));
            }
            ASTNode::Array(items) => {
                // For logical operators, we need to be careful with array items
                // especially empty arrays which are truthy in JSONLogic
                
                // Compile each item in reverse order for the stack
                let arg_count = items.len() as u32;

                // Special handling for each item to preserve arrays
                for item in items.iter().rev() {
                    match &**item {
                        // For array literals, we need to preserve them as-is
                        ASTNode::ArrayLiteral(array_items) => {
                            // Add the array directly to the constant pool
                            let array_data = self.arena.alloc(DataValue::Array(array_items));
                            let const_idx = self.const_pool.add(array_data)?;
                            self.emit(Instr::new(OpCode::LoadConst, const_idx));
                        },
                        // For nested arrays, recursively compile them
                        ASTNode::Array(nested_items) => {
                            // If it's an empty array, create it directly
                            if nested_items.is_empty() {
                                let empty_array = self.arena.alloc(DataValue::Array(&[]));
                                let const_idx = self.const_pool.add(empty_array)?;
                                self.emit(Instr::new(OpCode::LoadConst, const_idx));
                            } else {
                                // Otherwise compile each item and create an array
                                for nested_item in nested_items.iter().rev() {
                                    self.compile_without_return(nested_item)?;
                                }
                                self.emit(Instr::new(OpCode::Call, 0)); // Create array
                            }
                        },
                        // For other nodes, use standard compilation
                        _ => self.compile_without_return(item)?,
                    }
                }

                // Determine which logical operator to emit
                match op_type {
                    OperatorType::And => {
                        self.emit(Instr::new(
                            OpCode::Variadic,
                            (OpTag::And as u32) << 16 | arg_count,
                        ));
                    }
                    OperatorType::Or => {
                        self.emit(Instr::new(
                            OpCode::Variadic,
                            (OpTag::Or as u32) << 16 | arg_count,
                        ));
                    }
                    OperatorType::Not => {
                        if arg_count != 1 {
                            return Err(CompileError::LoweringError(format!(
                                "Expected 1 argument for not, got: {}",
                                arg_count
                            )));
                        }
                        self.emit(Instr::new(
                            OpCode::Variadic,
                            (OpTag::Not as u32) << 16 | arg_count,
                        ));
                    }
                    OperatorType::DoubleBang => {
                        if arg_count != 1 {
                            return Err(CompileError::LoweringError(format!(
                                "Expected 1 argument for double-bang, got: {}",
                                arg_count
                            )));
                        }
                        self.emit(Instr::new(
                            OpCode::Variadic,
                            (OpTag::DNot as u32) << 16 | arg_count,
                        ));
                    }
                    _ => {
                        return Err(CompileError::LoweringError(
                            "Unknown logical operator".into(),
                        ));
                    }
                }
            },
            ASTNode::ArrayLiteral(items) => {
                // For array literals, add the entire array to the constant pool
                let data = self.arena.alloc(DataValue::Array(items));
                let const_idx = self.const_pool.add(data)?;
                self.emit(Instr::new(OpCode::LoadConst, const_idx));

                // Determine which logical operator to emit
                match op_type {
                    OperatorType::And => {
                        self.emit(Instr::new(
                            OpCode::Variadic,
                            (OpTag::And as u32) << 16 | 1,
                        ));
                    }
                    OperatorType::Or => {
                        self.emit(Instr::new(
                            OpCode::Variadic,
                            (OpTag::Or as u32) << 16 | 1,
                        ));
                    }
                    OperatorType::Not => {
                        self.emit(Instr::new(
                            OpCode::Variadic,
                            (OpTag::Not as u32) << 16 | 1,
                        ));
                    }
                    OperatorType::DoubleBang => {
                        self.emit(Instr::new(
                            OpCode::Variadic,
                            (OpTag::DNot as u32) << 16 | 1,
                        ));
                    }
                    _ => {
                        return Err(CompileError::LoweringError(
                            "Unknown logical operator".into(),
                        ));
                    }
                }
            },
            _ => {
                return Err(CompileError::LoweringError(format!(
                    "Expected array of arguments, got: {:?}",
                    args
                )));
            }
        }

        Ok(())
    }

    // Helper method for compiling if operator arguments
    fn compile_if_args(&mut self, args: &'a ASTNode<'a>) -> Result<(), CompileError> {
        match args {
            ASTNode::Array(items) => {
                // Empty array - JSONLogic spec says this should return null
                if items.is_empty() {
                    let null_value = self.arena.alloc(DataValue::Null);
                    let const_idx = self.const_pool.add(null_value)?;
                    self.emit(Instr::new(OpCode::LoadConst, const_idx));
                    return Ok(());
                }
                
                // Single argument case - return the condition itself
                if items.len() == 1 {
                    self.compile_without_return(&items[0])?;
                    return Ok(());
                }

                // Handle chained if statements (if-else-if-else chain)
                // Format: [cond1, val1, cond2, val2, ..., default_val]
                
                // Keep track of all the jump-to-end points we need to update at the end
                let mut end_jumps = Vec::new();
                
                // Process all condition-value pairs
                let total_items = items.len();
                let mut i = 0;
                
                while i < total_items {
                    // If we only have one item left, it's the default value (else clause)
                    if i == total_items - 1 {
                        // Compile the default value
                        self.compile_without_return(&items[i])?;
                        break;
                    }
                    
                    // Compile the condition
                    self.compile_without_return(&items[i])?;
                    
                    // Emit JumpIfFalse to skip this value if condition is false
                    let jump_to_next_idx = self.emit_with_index(Instr::new(OpCode::JumpIfFalse, 0));
                    
                    // Compile the value for this condition
                    if i + 1 < total_items {
                        self.compile_without_return(&items[i + 1])?;
                    } else {
                        // This shouldn't happen with properly structured input
                        return Err(CompileError::LoweringError(
                            "If requires a value after each condition".to_string(),
                        ));
                    }
                    
                    // If this isn't the last condition-value pair, emit a jump to end
                    if i + 2 < total_items {
                        let jump_to_end_idx = self.emit_with_index(Instr::new(OpCode::Jump, 0));
                        end_jumps.push(jump_to_end_idx);
                    }
                    
                    // Update the jump_to_next offset to point to the next condition
                    let next_offset = self.instructions.len() as u32;
                    self.update_jump(jump_to_next_idx, next_offset);
                    
                    // Move to the next condition-value pair
                    i += 2;
                }
                
                // Update all jump-to-end offsets to point to the end
                let end_offset = self.instructions.len() as u32;
                for jump_idx in end_jumps {
                    self.update_jump(jump_idx, end_offset);
                }
            },
            ASTNode::ArrayLiteral(items) => {
                // Empty array - JSONLogic spec says this should return null
                if items.is_empty() {
                    let null_value = self.arena.alloc(DataValue::Null);
                    let const_idx = self.const_pool.add(null_value)?;
                    self.emit(Instr::new(OpCode::LoadConst, const_idx));
                    return Ok(());
                }
                
                // Single argument case - return the condition itself
                if items.len() == 1 {
                    let const_idx = self.const_pool.add(&items[0])?;
                    self.emit(Instr::new(OpCode::LoadConst, const_idx));
                    return Ok(());
                }
                
                // Handle chained if statements with array literals
                // Format: [cond1, val1, cond2, val2, ..., default_val]
                
                // Keep track of all the jump-to-end points we need to update at the end
                let mut end_jumps = Vec::new();
                
                // Process all condition-value pairs
                let total_items = items.len();
                let mut i = 0;
                
                while i < total_items {
                    // If we only have one item left, it's the default value (else clause)
                    if i == total_items - 1 {
                        // Add the default value to the constant pool and load it
                        let default_idx = self.const_pool.add(&items[i])?;
                        self.emit(Instr::new(OpCode::LoadConst, default_idx));
                        break;
                    }
                    
                    // Add the condition to the constant pool and load it
                    let cond_idx = self.const_pool.add(&items[i])?;
                    self.emit(Instr::new(OpCode::LoadConst, cond_idx));
                    
                    // Emit JumpIfFalse to skip this value if condition is false
                    let jump_to_next_idx = self.emit_with_index(Instr::new(OpCode::JumpIfFalse, 0));
                    
                    // Add the value for this condition to the constant pool and load it
                    if i + 1 < total_items {
                        let val_idx = self.const_pool.add(&items[i + 1])?;
                        self.emit(Instr::new(OpCode::LoadConst, val_idx));
                    } else {
                        // This shouldn't happen with properly structured input
                        return Err(CompileError::LoweringError(
                            "If requires a value after each condition".to_string(),
                        ));
                    }
                    
                    // If this isn't the last condition-value pair, emit a jump to end
                    if i + 2 < total_items {
                        let jump_to_end_idx = self.emit_with_index(Instr::new(OpCode::Jump, 0));
                        end_jumps.push(jump_to_end_idx);
                    }
                    
                    // Update the jump_to_next offset to point to the next condition
                    let next_offset = self.instructions.len() as u32;
                    self.update_jump(jump_to_next_idx, next_offset);
                    
                    // Move to the next condition-value pair
                    i += 2;
                }
                
                // Update all jump-to-end offsets to point to the end
                let end_offset = self.instructions.len() as u32;
                for jump_idx in end_jumps {
                    self.update_jump(jump_idx, end_offset);
                }
            },
            _ => {
                return Err(CompileError::LoweringError(
                    "If requires array arguments".to_string(),
                ));
            }
        }

        Ok(())
    }

    // Helper method for compiling var operator arguments
    fn compile_var_args(&mut self, args: &'a ASTNode<'a>) -> Result<(), CompileError> {
        match args {
            ASTNode::Array(items) if !items.is_empty() => {
                // Get the path from the first argument
                match &*items[0] {
                    ASTNode::Literal(path) => {
                        // Add path to constant pool
                        let path_idx = self.const_pool.add(path)?;

                        // Emit LoadVar instruction
                        self.emit(Instr::new(OpCode::LoadVar, path_idx));

                        // Handle default value if provided
                        if items.len() > 1 {
                            // TODO: Implement default value handling
                        }
                    }
                    _ => {
                        return Err(CompileError::LoweringError(
                            "Variable path must be a literal".to_string(),
                        ));
                    }
                }
            }
            ASTNode::Literal(path) => {
                // Direct path as argument
                let path_idx = self.const_pool.add(path)?;
                self.emit(Instr::new(OpCode::LoadVar, path_idx));
            }
            _ => {
                return Err(CompileError::LoweringError(
                    "Variable requires a path argument".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Compile a literal value
    fn compile_literal(&mut self, value: &'a DataValue<'a>) -> Result<(), CompileError> {
        // Add the literal value to the constant pool
        let const_idx = self.const_pool.add(value)?;

        // Emit a LoadConst instruction with the constant index
        self.emit(Instr::new(OpCode::LoadConst, const_idx));

        Ok(())
    }

    /// Ensure a Return instruction is emitted at the end if needed
    fn ensure_return(&mut self) {
        if !self.return_emitted
            && (self.instructions.is_empty()
                || self.instructions.last().unwrap().opcode() != OpCode::Return)
        {
            self.emit(Instr::new(OpCode::Return, 0));
            self.return_emitted = true;
        }
    }

    /// Get the number of instructions
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// Finalize and return both the instructions and constant pool values
    pub fn finalize(self) -> (&'a [Instr], Vec<DataValue<'a>>) {
        // Copy the instructions to the arena
        let instructions = self.arena.alloc_slice_copy(&self.instructions);

        // Get the values from the constant pool
        let const_values = self.const_pool.finalize();

        (instructions, const_values)
    }

    /// Emit an instruction
    pub fn emit(&mut self, instr: Instr) {
        self.instructions.push(instr);
    }

    /// Emit an instruction and return its index
    pub fn emit_with_index(&mut self, instr: Instr) -> usize {
        let idx = self.instructions.len();
        self.instructions.push(instr);
        idx
    }

    /// Update a jump instruction with the correct offset
    pub fn update_jump(&mut self, jump_idx: usize, target_offset: u32) {
        if let Some(instr) = self.instructions.get_mut(jump_idx) {
            instr.set_operand(target_offset);
        } else {
            panic!("Jump index out of bounds: {}", jump_idx);
        }
    }

    /// Allocate a new local variable
    pub fn alloc_local(&mut self) -> Result<u32, CompileError> {
        if self.locals_count >= self.locals_limit {
            return Err(CompileError::LoweringError(
                "Local variable limit exceeded".to_string(),
            ));
        }

        let idx = self.locals_count;
        self.locals_count += 1;
        Ok(idx)
    }

    // Helper method for compiling merge operator arguments
    fn compile_merge_args(&mut self, args: &'a ASTNode<'a>) -> Result<(), CompileError> {
        match args {
            ASTNode::Array(items) => {
                // Compile each item in reverse order for the stack
                for item in items.iter().rev() {
                    self.compile_without_return(item)?;
                }
                
                // Emit a Call instruction with Merge tag (1)
                self.emit(Instr::new(OpCode::Call, CallTag::Merge as u32));
                
                Ok(())
            },
            ASTNode::ArrayLiteral(items) => {
                // For array literals, add each item to the constant pool
                for item in items.iter().rev() {
                    let const_idx = self.const_pool.add(item)?;
                    self.emit(Instr::new(OpCode::LoadConst, const_idx));
                }
                
                // Emit a Call instruction with Merge tag (1)
                self.emit(Instr::new(OpCode::Call, CallTag::Merge as u32));
                
                Ok(())
            },
            ASTNode::Literal(value) => {
                let const_idx = self.const_pool.add(value)?;
                self.emit(Instr::new(OpCode::LoadConst, const_idx));
                self.emit(Instr::new(OpCode::Call, CallTag::Merge as u32));
                Ok(())
            },
            _ => {
                Err(CompileError::LoweringError(format!(
                    "Merge requires array arguments, got: {:?}",
                    args
                )))
            }
        }
    }
    
    // Helper method for compiling cat operator arguments
    fn compile_cat_args(&mut self, args: &'a ASTNode<'a>) -> Result<(), CompileError> {
        match args {
            ASTNode::Array(items) => {
                // Compile each item in reverse order for the stack
                for item in items.iter().rev() {
                    self.compile_without_return(item)?;
                }
                
                // Emit a Call instruction with Cat tag (7)
                self.emit(Instr::new(OpCode::Call, CallTag::Cat as u32));
                
                Ok(())
            },
            ASTNode::ArrayLiteral(items) => {
                // For array literals, add each item to the constant pool
                for item in items.iter().rev() {
                    let const_idx = self.const_pool.add(item)?;
                    self.emit(Instr::new(OpCode::LoadConst, const_idx));
                }
                
                // Emit a Call instruction with Cat tag (7)
                self.emit(Instr::new(OpCode::Call, CallTag::Cat as u32));
                
                Ok(())
            },
            ASTNode::Literal(value) => {
                let const_idx = self.const_pool.add(value)?;
                self.emit(Instr::new(OpCode::LoadConst, const_idx));
                self.emit(Instr::new(OpCode::Call, CallTag::Cat as u32));
                Ok(())
            },
            _ => {
                Err(CompileError::LoweringError(format!(
                    "Cat requires array arguments, got: {:?}",
                    args
                )))
            }
        }
    }
    
    // Helper method for compiling substr operator arguments
    fn compile_substr_args(&mut self, args: &'a ASTNode<'a>) -> Result<(), CompileError> {
        match args {
            ASTNode::Array(items) => {
                // Check if we have at least the string and start arguments
                if items.len() < 2 {
                    return Err(CompileError::LoweringError(
                        "Substr requires at least 2 arguments: string and start".to_string(),
                    ));
                }
                
                // IMPORTANT: Compile in correct order for the stack
                // The substr VM implementation expects items in this order (bottom to top of stack):
                // 1. string
                // 2. start
                // 3. length (optional)
                
                // Process items in normal order (not reversed)
                if items.len() >= 3 {
                    // If we have length, compile it first (it will be at the top of the stack)
                    self.compile_without_return(&items[2])?;
                }
                
                // Then start (middle of stack)
                self.compile_without_return(&items[1])?;
                
                // Then string (bottom of stack)
                self.compile_without_return(&items[0])?;
                
                // Emit a Call instruction with Substr tag (8)
                self.emit(Instr::new(OpCode::Call, CallTag::Substring as u32));
                
                Ok(())
            },
            ASTNode::ArrayLiteral(items) => {
                // Check if we have at least the string and start arguments
                if items.len() < 2 {
                    return Err(CompileError::LoweringError(
                        "Substr requires at least 2 arguments: string and start".to_string(),
                    ));
                }
                
                // IMPORTANT: Load constants in correct order for the stack
                // The substr VM implementation expects items in this order (bottom to top of stack):
                // 1. string
                // 2. start
                // 3. length (optional)
                
                // Process constants in normal order (not reversed)
                if items.len() >= 3 {
                    // If we have length, load it first (it will be at the top of the stack)
                    let const_idx = self.const_pool.add(&items[2])?;
                    self.emit(Instr::new(OpCode::LoadConst, const_idx));
                }
                
                // Then start (middle of stack)
                let const_idx = self.const_pool.add(&items[1])?;
                self.emit(Instr::new(OpCode::LoadConst, const_idx));
                
                // Then string (bottom of stack)
                let const_idx = self.const_pool.add(&items[0])?;
                self.emit(Instr::new(OpCode::LoadConst, const_idx));
                
                // Emit a Call instruction with Substr tag (8)
                self.emit(Instr::new(OpCode::Call, CallTag::Substring as u32));
                
                Ok(())
            },
            _ => {
                Err(CompileError::LoweringError(format!(
                    "Substr requires array arguments, got: {:?}",
                    args
                )))
            }
        }
    }

    // Helper method for compiling missing operator arguments
    fn compile_missing_args(&mut self, args: &'a ASTNode<'a>) -> Result<(), CompileError> {
        match args {
            ASTNode::Array(items) => {
                // Compile each path argument in reverse order for the stack
                for item in items.iter().rev() {
                    self.compile_without_return(item)?;
                }
                
                // Emit a Call instruction with Missing tag
                self.emit(Instr::new(OpCode::Call, CallTag::Missing as u32));
                
                Ok(())
            },
            ASTNode::ArrayLiteral(items) => {
                // For array literals, add each path to the constant pool
                for item in items.iter().rev() {
                    let const_idx = self.const_pool.add(item)?;
                    self.emit(Instr::new(OpCode::LoadConst, const_idx));
                }
                
                // Emit a Call instruction with Missing tag
                self.emit(Instr::new(OpCode::Call, CallTag::Missing as u32));
                
                Ok(())
            },
            ASTNode::Literal(value) => {
                // Handle a direct literal value (usually a string or array)
                let const_idx = self.const_pool.add(value)?;
                self.emit(Instr::new(OpCode::LoadConst, const_idx));
                self.emit(Instr::new(OpCode::Call, CallTag::Missing as u32));
                Ok(())
            },
            // Support complex expressions (merge, if, etc.) that evaluate to an array
            ASTNode::Operator { .. } => {
                // Compile the expression which should produce an array on the stack
                self.compile_without_return(args)?;
                
                // Now call the Missing operator on this array
                self.emit(Instr::new(OpCode::Call, CallTag::Missing as u32));
                
                Ok(())
            },
            _ => {
                Err(CompileError::LoweringError(format!(
                    "Missing requires an array or an expression that evaluates to an array of paths, got: {:?}",
                    args
                )))
            }
        }
    }
    
    // Helper method for compiling missing_some operator arguments
    fn compile_missing_some_args(&mut self, args: &'a ASTNode<'a>) -> Result<(), CompileError> {
        match args {
            ASTNode::Array(items) => {
                // Need at least 2 arguments: minimum count and array of paths
                if items.len() < 2 {
                    return Err(CompileError::LoweringError(
                        "missing_some requires at least 2 arguments: min count and path array".to_string()
                    ));
                }
                
                // Compile the second argument (array of paths) first
                self.compile_without_return(&items[1])?;
                
                // Then compile the first argument (minimum required count)
                self.compile_without_return(&items[0])?;
                
                // Emit a Call instruction with MissingSome tag
                self.emit(Instr::new(OpCode::Call, CallTag::MissingSome as u32));
                
                Ok(())
            },
            ASTNode::ArrayLiteral(items) => {
                // Need at least 2 arguments: minimum count and array of paths
                if items.len() < 2 {
                    return Err(CompileError::LoweringError(
                        "missing_some requires at least 2 arguments: min count and path array".to_string()
                    ));
                }
                
                // Load each argument from constant pool in reverse order
                for item in items.iter().rev() {
                    let const_idx = self.const_pool.add(item)?;
                    self.emit(Instr::new(OpCode::LoadConst, const_idx));
                }
                
                // Emit a Call instruction with MissingSome tag
                self.emit(Instr::new(OpCode::Call, CallTag::MissingSome as u32));
                
                Ok(())
            },
            // Support the case where the second argument is a complex expression
            ASTNode::Operator { op_type: _, args: _ } => {
                // We expect this to be a complex expression that returns a 2-element array
                // [min_count, paths_array]
                // Compile the expression directly
                self.compile_without_return(args)?;
                
                // Call the MissingSome operator on the resulting array
                self.emit(Instr::new(OpCode::Call, CallTag::MissingSome as u32));
                
                Ok(())
            },
            _ => {
                Err(CompileError::LoweringError(format!(
                    "missing_some requires an array [min_count, paths] or an expression that evaluates to such an array, got: {:?}",
                    args
                )))
            }
        }
    }
}
