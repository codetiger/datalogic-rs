//! Stack-based instruction execution for JSONLogic
//!
//! This module provides the stack data structure and operations for
//! non-recursive evaluation of JSONLogic expressions.

use crate::operators::arithmetic;
use crate::operators::collection;
use crate::operators::comparison;
use crate::operators::logic;
use crate::operators::misc;
use crate::operators::string;
use crate::parser::{ASTNode, EvaluationStrategy, OperatorType};
use bumpalo::collections::Vec as BumpVec;
use bumpalo::Bump;
use datavalue_rs::Number;
use datavalue_rs::{DataValue, Result};

/// Represents an instruction on the evaluation stack
#[derive(Debug, Clone)]
pub enum Instruction<'a> {
    /// Evaluate a token
    Eval(&'a ASTNode<'a>),

    /// Collect array items from the value stack
    Array(usize),

    /// Collect operator arguments from the value stack
    OpArgs(OperatorType, usize),

    /// Evaluate a lazy operator with its raw arguments token
    LazyOp(OperatorType, &'a ASTNode<'a>),

    /// Create an array directly from literal values (optimization)
    ArrayLit(&'a [DataValue<'a>]),
}

/// A stack of instructions for evaluating JSONLogic expressions
#[derive(Debug)]
pub struct InstructionStack<'a> {
    pub instructions: Vec<Instruction<'a>>,
}

impl<'a> InstructionStack<'a> {
    /// Creates a new instruction stack with the given token as the root
    pub fn new(token: &'a ASTNode<'a>) -> Self {
        let instructions = vec![Instruction::Eval(token)];

        Self { instructions }
    }

    /// Generates the instruction stack without executing it
    ///
    /// This allows precompilation of the instructions, which can be stored
    /// and reused multiple times with different data contexts.
    pub fn compile(&mut self) -> Result<()> {
        // A temporary vector to store the compiled instructions in correct order
        let mut compiled_instructions = Vec::new();
        let mut stack_index = self.instructions.len();

        // Process tokens until we have a complete instruction set
        while stack_index > 0 {
            let instruction = self.instructions[stack_index - 1].clone();
            stack_index -= 1;
            match instruction {
                Instruction::Eval(token) => {
                    // For evaluation instructions, we need to process the token
                    // to build up the instruction stack
                    match token {
                        ASTNode::Literal(_) => {
                            // Literals just become a single instruction
                            compiled_instructions.push(instruction);
                        }

                        ASTNode::ArrayLiteral(items) => {
                            // Optimization: directly create array instead of pushing items + collecting
                            compiled_instructions.push(Instruction::ArrayLit(items));
                        }

                        ASTNode::Operator { op_type, args } => {
                            // Ensure all operators use their evaluation strategy without exceptions
                            match op_type.evaluation_strategy() {
                                EvaluationStrategy::Eager => {
                                    match &**args {
                                        ASTNode::ArrayLiteral(items) => {
                                            let count = items.len();
                                            // Add operator and all items as separate instructions
                                            compiled_instructions
                                                .push(Instruction::OpArgs(*op_type, count));
                                            compiled_instructions.push(instruction);
                                        }
                                        ASTNode::Array(items) => {
                                            let count = items.len();
                                            // Add operator and instructions to evaluate all items
                                            compiled_instructions
                                                .push(Instruction::OpArgs(*op_type, count));
                                            for item in items.iter() {
                                                compiled_instructions.push(Instruction::Eval(item));
                                            }
                                        }
                                        _ => {
                                            compiled_instructions
                                                .push(Instruction::OpArgs(*op_type, 1));
                                            compiled_instructions.push(Instruction::Eval(args));
                                        }
                                    }
                                }
                                EvaluationStrategy::Lazy => {
                                    // For lazy operators, add special instruction
                                    compiled_instructions.push(Instruction::LazyOp(*op_type, args));
                                }
                            }
                        }

                        ASTNode::Array(items) => {
                            let count = items.len();
                            compiled_instructions.push(Instruction::Array(count));
                            for item in items.iter() {
                                compiled_instructions.push(Instruction::Eval(item));
                            }
                        }

                        ASTNode::Variable { .. } => {
                            compiled_instructions.push(instruction);
                        }

                        ASTNode::DynamicVariable { .. } => {
                            compiled_instructions.push(instruction);
                        }

                        ASTNode::CustomOperator { .. } => {
                            compiled_instructions.push(instruction);
                        }
                    }
                }

                // For other instruction types, add them directly
                _ => compiled_instructions.push(instruction),
            }
        }

        // Now we have all the compiled instructions; we need to reverse them
        // as they will be executed in LIFO order
        compiled_instructions.reverse();

        // Store the compiled instructions
        self.instructions = compiled_instructions.into_iter().collect();

        Ok(())
    }

    /// Evaluates the instruction stack
    pub fn evaluate(&self, data: &'a DataValue<'a>, arena: &'a Bump) -> Result<&'a DataValue<'a>> {
        let mut values = BumpVec::new_in(arena);
        let mut stack_index = self.instructions.len();

        // Continue processing instructions until the stack is empty
        while stack_index > 0 {
            let instruction = &self.instructions[stack_index - 1];
            stack_index -= 1;
            match instruction {
                Instruction::Eval(token) => {
                    self.process_token(&mut values, token, data, arena)?;
                }
                Instruction::Array(count) => {
                    self.collect_array(&mut values, *count, arena)?;
                }
                Instruction::OpArgs(op_type, count) => {
                    self.collect_operator_args(&mut values, *op_type, *count, data, arena)?;
                }
                Instruction::LazyOp(op_type, args) => {
                    let result = self.evaluate_lazy_operator(*op_type, args, data, arena)?;
                    values.push(result);
                }
                Instruction::ArrayLit(items) => {
                    // Optimization: create array directly from literal values
                    let array_values: Vec<DataValue<'a>> = items.to_vec();
                    let array = DataValue::Array(arena.alloc_slice_fill_iter(array_values));
                    values.push(arena.alloc(array));
                }
            }
        }

        // The final result should be the only value on the stack
        if values.len() == 1 {
            Ok(values.pop().unwrap())
        } else {
            Err(datavalue_rs::Error::Custom(format!(
                "Invalid evaluation result: {} values on stack",
                values.len()
            )))
        }
    }

    /// Processes a token and adds appropriate values to the stack
    fn process_token(
        &self,
        values: &mut BumpVec<&'a DataValue<'a>>,
        token: &'a ASTNode<'a>,
        data: &'a DataValue<'a>,
        arena: &'a Bump,
    ) -> Result<()> {
        match token {
            // For literal values, just push them onto the value stack
            ASTNode::Literal(value) => {
                values.push(value);
            }

            ASTNode::ArrayLiteral(items) => {
                // Optimization: create array directly rather than pushing items and collecting
                let array_values: Vec<DataValue<'a>> = items.to_vec();
                let array = DataValue::Array(arena.alloc_slice_fill_iter(array_values));
                values.push(arena.alloc(array));
            }

            // For operators, handle based on the type and evaluation strategy
            ASTNode::Operator { op_type, args } => {
                // Check the evaluation strategy for this operator
                match op_type.evaluation_strategy() {
                    EvaluationStrategy::Eager => {
                        match &**args {
                            ASTNode::ArrayLiteral(items) => {
                                // For ArrayLiteral, just push the items directly and apply the operator
                                for item in items {
                                    values.push(item);
                                }
                                self.collect_operator_args(
                                    values,
                                    *op_type,
                                    items.len(),
                                    data,
                                    arena,
                                )?;
                            }
                            ASTNode::Array(items) => {
                                // For nested tokens, process each one first
                                for item in items {
                                    self.process_token(values, item, data, arena)?;
                                }
                                self.collect_operator_args(
                                    values,
                                    *op_type,
                                    items.len(),
                                    data,
                                    arena,
                                )?;
                            }
                            // Regular handling for single argument operators
                            _ => {
                                // Process the single argument
                                self.process_token(values, args, data, arena)?;
                                self.collect_operator_args(values, *op_type, 1, data, arena)?;
                            }
                        }
                    }
                    EvaluationStrategy::Lazy => {
                        // For lazy operators, evaluate them directly
                        let result = self.evaluate_lazy_operator(*op_type, args, data, arena)?;
                        values.push(result);
                    }
                }
            }

            // For arrays, evaluate each item and collect the results
            ASTNode::Array(items) => {
                for item in items {
                    self.process_token(values, item, data, arena)?;
                }
                // Collect these results into an array
                self.collect_array(values, items.len(), arena)?;
            }

            // Handle variables
            ASTNode::Variable {
                path,
                default,
                scope_jump,
            } => {
                self.evaluate_variable(values, path, default, scope_jump, data, arena)?;
            }

            ASTNode::DynamicVariable {
                path_expr,
                default,
                scope_jump,
            } => {
                // First evaluate the path expression to get the actual path
                let path_value = self.evaluate_token(path_expr, data, arena)?;

                // For a dynamic variable, evaluate the default expression if provided
                let evaluated_default = match default {
                    Some(def_expr) => {
                        let def_value = self.evaluate_token(def_expr, data, arena)?;
                        Some(def_value)
                    }
                    None => None,
                };

                // Convert string paths like "pie.filling" to a proper path array
                let path = match path_value {
                    DataValue::String(s) if s.contains('.') => {
                        // Split the string by dots and create a path array
                        let parts: Vec<&str> = s.split('.').collect();
                        let path_parts: Vec<DataValue> = parts
                            .into_iter()
                            .map(|part| DataValue::String(arena.alloc_str(part)))
                            .collect();
                        arena.alloc(DataValue::Array(arena.alloc_slice_fill_iter(path_parts)))
                    }
                    _ => path_value,
                };

                // Then handle it like a regular variable
                self.evaluate_variable(values, path, &evaluated_default, scope_jump, data, arena)?;
            }

            ASTNode::CustomOperator { .. } => {
                return Err(datavalue_rs::Error::Custom(
                    "Custom operators not yet implemented".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Evaluates a lazy operator with its raw arguments
    fn evaluate_lazy_operator(
        &self,
        op_type: OperatorType,
        args: &'a ASTNode<'a>,
        data: &'a DataValue<'a>,
        arena: &'a Bump,
    ) -> Result<&'a DataValue<'a>> {
        // Call the appropriate lazy operator function
        let result = match op_type {
            // Logic operators
            OperatorType::If => logic::evaluate_if(args, data, arena)?,
            OperatorType::And => logic::evaluate_and(args, data, arena)?,
            OperatorType::Or => logic::evaluate_or(args, data, arena)?,
            OperatorType::NullCoalesce => logic::evaluate_null_coalesce(args, data, arena)?,

            // Comparison operators
            OperatorType::Equal => comparison::evaluate_equal(args, data, arena)?,
            OperatorType::StrictEqual => comparison::evaluate_strict_equal(args, data, arena)?,
            OperatorType::NotEqual => comparison::evaluate_not_equal(args, data, arena)?,
            OperatorType::StrictNotEqual => {
                comparison::evaluate_strict_not_equal(args, data, arena)?
            }
            OperatorType::GT => comparison::evaluate_gt(args, data, arena)?,
            OperatorType::LT => comparison::evaluate_lt(args, data, arena)?,
            OperatorType::GTE => comparison::evaluate_gte(args, data, arena)?,
            OperatorType::LTE => comparison::evaluate_lte(args, data, arena)?,
            OperatorType::In => comparison::evaluate_in(args, data, arena)?,

            // Collection operators
            OperatorType::Filter => collection::evaluate_filter(args, data, arena)?,
            OperatorType::Map => collection::evaluate_map(args, data, arena)?,
            OperatorType::All => collection::evaluate_all(args, data, arena)?,
            OperatorType::Some => collection::evaluate_some(args, data, arena)?,
            OperatorType::None => collection::evaluate_none(args, data, arena)?,
            OperatorType::Reduce => collection::evaluate_reduce(args, data, arena)?,

            OperatorType::Merge => collection::evaluate_merge(args, data, arena)?,
            OperatorType::Cat => collection::evaluate_cat(args, data, arena)?,

            // For other lazy operators that might be implemented later
            _ => {
                return Err(datavalue_rs::Error::Custom(format!(
                    "Lazy operator {:?} not yet implemented",
                    op_type
                )));
            }
        };

        Ok(result)
    }

    /// Collects array items from the value stack
    fn collect_array(
        &self,
        values: &mut BumpVec<&'a DataValue<'a>>,
        count: usize,
        arena: &'a Bump,
    ) -> Result<()> {
        if values.len() < count {
            return Err(datavalue_rs::Error::Custom(format!(
                "Not enough values on stack for array: need {}, have {}",
                count,
                values.len()
            )));
        }

        // Get the array items from the stack
        let start = values.len() - count;
        let array_items: Vec<_> = values.drain(start..).collect();

        // Create the array value
        // Convert to the expected DataValue type
        let array_values: Vec<DataValue<'a>> = array_items.iter().map(|&v| v.clone()).collect();

        let array = DataValue::Array(arena.alloc_slice_fill_iter(array_values));
        values.push(arena.alloc(array));

        Ok(())
    }

    /// Collects operator arguments and applies the operator
    fn collect_operator_args(
        &self,
        values: &mut BumpVec<&'a DataValue<'a>>,
        op_type: OperatorType,
        count: usize,
        data: &'a DataValue<'a>,
        arena: &'a Bump,
    ) -> Result<()> {
        if values.len() < count {
            return Err(datavalue_rs::Error::Custom(format!(
                "Not enough values on stack for operator: need {}, have {}",
                count,
                values.len()
            )));
        }

        // Get the operator arguments from the stack
        let start = values.len() - count;
        let arg_refs: Vec<_> = values.drain(start..).collect();

        // Convert references to DataValue objects
        let args: Vec<DataValue<'a>> = arg_refs.iter().map(|&v| v.clone()).collect();

        // Apply the operator based on its type
        let result = match op_type {
            OperatorType::Add => arithmetic::evaluate_add(&args, arena)?,
            OperatorType::Subtract => arithmetic::evaluate_subtract(&args, arena)?,
            OperatorType::Multiply => arithmetic::evaluate_multiply(&args, arena)?,
            OperatorType::Divide => arithmetic::evaluate_divide(&args, arena)?,
            OperatorType::Modulo => arithmetic::evaluate_modulo(&args, arena)?,
            OperatorType::Min => arithmetic::evaluate_min(&args, arena)?,
            OperatorType::Max => arithmetic::evaluate_max(&args, arena)?,
            OperatorType::Abs => arithmetic::evaluate_abs(&args, arena)?,
            OperatorType::Ceil => arithmetic::evaluate_ceil(&args, arena)?,
            OperatorType::Floor => arithmetic::evaluate_floor(&args, arena)?,
            OperatorType::Missing => misc::evaluate_missing_args(&args, data, arena)?,
            OperatorType::MissingSome => misc::evaluate_missing_some_args(&args, data, arena)?,
            OperatorType::Exists => misc::evaluate_exists_args(&args, data, arena)?,
            OperatorType::Substring => string::evaluate_substring(&args, arena)?,
            OperatorType::Not => logic::evaluate_not(&args, arena)?,
            OperatorType::DoubleBang => logic::evaluate_double_bang(&args, arena)?,
            _ => {
                return Err(datavalue_rs::Error::Custom(format!(
                    "Operator {:?} not yet implemented in stack engine",
                    op_type
                )));
            }
        };

        values.push(result);
        Ok(())
    }

    fn evaluate_variable(
        &self,
        values: &mut BumpVec<&'a DataValue<'a>>,
        path: &'a DataValue<'a>,
        default: &Option<&'a DataValue<'a>>,
        scope_jump: &Option<usize>,
        data: &'a DataValue<'a>,
        arena: &'a Bump,
    ) -> Result<()> {
        if scope_jump.is_none() {
            match path {
                DataValue::Array([]) | DataValue::Null => {
                    values.push(data);
                }
                DataValue::Array(items) => {
                    let mut context = Some(data);
                    for item in items.iter() {
                        match item {
                            DataValue::String(key) => {
                                context = context.unwrap_or(&DataValue::Null).get(key)
                            }
                            DataValue::Number(Number::Integer(i)) => {
                                context = context.unwrap_or(&DataValue::Null).get_index(*i as usize)
                            }
                            _ => unreachable!(),
                        }
                    }
                    if let Some(context) = context {
                        values.push(context);
                    } else if let Some(default) = default {
                        values.push(*default);
                    } else {
                        values.push(arena.alloc(DataValue::Null));
                    }
                }
                DataValue::String(key) => {
                    let value = data.get(key);
                    if let Some(value) = value {
                        values.push(value);
                    } else if let Some(default) = default {
                        values.push(*default);
                    } else {
                        values.push(arena.alloc(DataValue::Null));
                    }
                }
                DataValue::Number(Number::Integer(i)) => {
                    let value = data.get_index(*i as usize);
                    if let Some(value) = value {
                        values.push(value);
                    } else if let Some(default) = default {
                        values.push(*default);
                    } else {
                        values.push(arena.alloc(DataValue::Null));
                    }
                }
                _ => {
                    return Err(datavalue_rs::Error::Custom(format!(
                        "Invalid path: {:?}",
                        path
                    )))
                }
            }
        }
        Ok(())
    }

    /// Evaluates a token and returns its value
    fn evaluate_token(
        &self,
        token: &'a ASTNode<'a>,
        data: &'a DataValue<'a>,
        arena: &'a Bump,
    ) -> Result<&'a DataValue<'a>> {
        // Create a temporary instruction stack to evaluate the token
        let temp_stack = InstructionStack::new(token);

        // Evaluate the token and return the result
        temp_stack.evaluate(data, arena)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ASTNode;
    use datavalue_rs::helpers;

    #[test]
    fn test_simple_addition() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a simple addition: 1 + 2 + 3
        let arg1 = Box::new(ASTNode::Literal(helpers::int(1)));
        let arg2 = Box::new(ASTNode::Literal(helpers::int(2)));
        let arg3 = Box::new(ASTNode::Literal(helpers::int(3)));

        let args = Box::new(ASTNode::Array(vec![arg1, arg2, arg3]));
        let add_token = ASTNode::Operator {
            op_type: OperatorType::Add,
            args,
        };

        // Evaluate using our stack-based engine
        let stack = InstructionStack::new(&add_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(6));
    }

    #[test]
    fn test_nested_addition() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a nested addition: (1 + 2) + (3 + 4)
        let inner_args1 = Box::new(ASTNode::ArrayLiteral(vec![
            DataValue::Number(Number::Integer(1)),
            DataValue::Number(Number::Integer(2)),
        ]));

        let inner_args2 = Box::new(ASTNode::ArrayLiteral(vec![
            DataValue::Number(Number::Integer(3)),
            DataValue::Number(Number::Integer(4)),
        ]));

        let inner_add1 = Box::new(ASTNode::Operator {
            op_type: OperatorType::Add,
            args: inner_args1,
        });

        let inner_add2 = Box::new(ASTNode::Operator {
            op_type: OperatorType::Add,
            args: inner_args2,
        });

        let args = Box::new(ASTNode::Array(vec![inner_add1, inner_add2]));
        let add_token = ASTNode::Operator {
            op_type: OperatorType::Add,
            args,
        };

        // Evaluate using our stack-based engine
        let stack = InstructionStack::new(&add_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(10));
    }

    #[test]
    fn test_subtract() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a subtraction: 10 - 3 - 2
        let arg1 = Box::new(ASTNode::Literal(helpers::int(10)));
        let arg2 = Box::new(ASTNode::Literal(helpers::int(3)));
        let arg3 = Box::new(ASTNode::Literal(helpers::int(2)));

        let args = Box::new(ASTNode::Array(vec![arg1, arg2, arg3]));
        let subtract_token = ASTNode::Operator {
            op_type: OperatorType::Subtract,
            args,
        };

        // Evaluate using our stack-based engine
        let stack = InstructionStack::new(&subtract_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(5)); // 10 - 3 - 2 = 5
    }

    #[test]
    fn test_multiply() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        let args = Box::new(ASTNode::ArrayLiteral(vec![
            DataValue::Number(Number::Integer(2)),
            DataValue::Number(Number::Integer(3)),
            DataValue::Number(Number::Integer(4)),
        ]));
        let multiply_token = ASTNode::Operator {
            op_type: OperatorType::Multiply,
            args,
        };

        // Evaluate using our stack-based engine
        let stack = InstructionStack::new(&multiply_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(24)); // 2 * 3 * 4 = 24
    }

    #[test]
    fn test_divide() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        let args = Box::new(ASTNode::ArrayLiteral(vec![
            DataValue::Number(Number::Integer(20)),
            DataValue::Number(Number::Integer(4)),
            DataValue::Number(Number::Integer(2)),
        ]));
        let divide_token = ASTNode::Operator {
            op_type: OperatorType::Divide,
            args,
        };

        // Evaluate using our stack-based engine
        let stack = InstructionStack::new(&divide_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::float(2.5)); // 20 / 4 / 2 = 2.5
    }

    #[test]
    fn test_modulo() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        let args = Box::new(ASTNode::ArrayLiteral(vec![
            DataValue::Number(Number::Integer(17)),
            DataValue::Number(Number::Integer(5)),
        ]));
        let modulo_token = ASTNode::Operator {
            op_type: OperatorType::Modulo,
            args,
        };

        // Evaluate using our stack-based engine
        let stack = InstructionStack::new(&modulo_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(2)); // 17 % 5 = 2
    }

    #[test]
    fn test_min() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a custom min operator: min(5, 2, 8, 1, 9)
        let min_token = ASTNode::Operator {
            op_type: OperatorType::Min,
            args: Box::new(ASTNode::ArrayLiteral(vec![
                DataValue::Number(Number::Integer(5)),
                DataValue::Number(Number::Integer(2)),
                DataValue::Number(Number::Integer(8)),
                DataValue::Number(Number::Integer(1)),
                DataValue::Number(Number::Integer(9)),
            ])),
        };

        // Evaluate using our stack-based engine
        let stack = InstructionStack::new(&min_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(1)); // min(5, 2, 8, 1, 9) = 1
    }

    #[test]
    fn test_max() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a custom max operator: max(5, 2, 8, 1, 9)
        let max_token = ASTNode::Operator {
            op_type: OperatorType::Max,
            args: Box::new(ASTNode::ArrayLiteral(vec![
                DataValue::Number(Number::Integer(5)),
                DataValue::Number(Number::Integer(2)),
                DataValue::Number(Number::Integer(8)),
                DataValue::Number(Number::Integer(1)),
                DataValue::Number(Number::Integer(9)),
            ])),
        };

        // Evaluate using our stack-based engine
        let stack = InstructionStack::new(&max_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(9)); // max(5, 2, 8, 1, 9) = 9
    }

    #[test]
    fn test_abs() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a custom abs operator: abs(-5, 2, 8, 1, 9)
        let abs_token = ASTNode::Operator {
            op_type: OperatorType::Abs,
            args: Box::new(ASTNode::ArrayLiteral(vec![
                DataValue::Number(Number::Integer(-5)),
                DataValue::Number(Number::Integer(2)),
                DataValue::Number(Number::Integer(8)),
                DataValue::Number(Number::Integer(1)),
                DataValue::Number(Number::Integer(9)),
            ])),
        };

        // Evaluate using our stack-based engine
        let stack = InstructionStack::new(&abs_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(
            result,
            &helpers::array(
                &arena,
                vec![
                    helpers::int(5),
                    helpers::int(2),
                    helpers::int(8),
                    helpers::int(1),
                    helpers::int(9)
                ]
            )
        );
    }

    #[test]
    fn test_ceil() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a custom ceil operator: ceil(1.2, 2.7, 3.8, 4.3)
        let ceil_token = ASTNode::Operator {
            op_type: OperatorType::Ceil,
            args: Box::new(ASTNode::ArrayLiteral(vec![
                DataValue::Number(Number::Float(1.2)),
                DataValue::Number(Number::Float(2.7)),
                DataValue::Number(Number::Float(3.8)),
                DataValue::Number(Number::Float(4.3)),
            ])),
        };

        // Evaluate using our stack-based engine
        let stack = InstructionStack::new(&ceil_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(
            result,
            &helpers::array(
                &arena,
                vec![
                    helpers::float(2.0),
                    helpers::float(3.0),
                    helpers::float(4.0),
                    helpers::float(5.0)
                ]
            )
        );
    }

    #[test]
    fn test_floor() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a custom floor operator: floor(1.2, 2.7, 3.8, 4.3)
        let floor_token = ASTNode::Operator {
            op_type: OperatorType::Floor,
            args: Box::new(ASTNode::ArrayLiteral(vec![
                DataValue::Number(Number::Float(1.2)),
                DataValue::Number(Number::Float(2.7)),
                DataValue::Number(Number::Float(3.8)),
                DataValue::Number(Number::Float(4.3)),
            ])),
        };

        // Evaluate using our stack-based engine
        let stack = InstructionStack::new(&floor_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(
            result,
            &helpers::array(
                &arena,
                vec![
                    helpers::float(1.0),
                    helpers::float(2.0),
                    helpers::float(3.0),
                    helpers::float(4.0)
                ]
            )
        );
    }
}
