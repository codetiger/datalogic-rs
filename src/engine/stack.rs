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
use crate::parser::{EvaluationStrategy, OperatorType, Token};
use bumpalo::Bump;
use datavalue_rs::Number;
use datavalue_rs::{DataValue, Result};

/// Represents an instruction on the evaluation stack
#[derive(Debug)]
enum Instruction<'a> {
    /// Evaluate a token
    Evaluate(&'a Token<'a>),

    /// Collect array items from the value stack
    CollectArray(usize),

    /// Collect operator arguments from the value stack
    CollectOperatorArgs(OperatorType, usize),

    /// Evaluate a lazy operator with its raw arguments token
    EvaluateLazyOperator(OperatorType, &'a Token<'a>),
}

/// A stack of instructions for evaluating JSONLogic expressions
pub struct InstructionStack<'a> {
    instructions: Vec<Instruction<'a>>,
    values: Vec<&'a DataValue<'a>>,
    data: Option<&'a DataValue<'a>>,
}

impl<'a> InstructionStack<'a> {
    /// Creates a new instruction stack with the given token as the root
    pub fn new(token: &'a Token<'a>) -> Self {
        let instructions = vec![Instruction::Evaluate(token)];

        Self {
            instructions,
            values: Vec::new(),
            data: None,
        }
    }

    /// Evaluates the instruction stack
    pub fn evaluate(
        &mut self,
        data: &'a DataValue<'a>,
        arena: &'a Bump,
    ) -> Result<&'a DataValue<'a>> {
        // Store data context for lazy operators
        self.data = Some(data);

        // Continue processing instructions until the stack is empty
        while let Some(instruction) = self.instructions.pop() {
            match instruction {
                Instruction::Evaluate(token) => {
                    self.process_token(token, data, arena)?;
                }
                Instruction::CollectArray(count) => {
                    self.collect_array(count, arena)?;
                }
                Instruction::CollectOperatorArgs(op_type, count) => {
                    self.collect_operator_args(op_type, count, arena)?;
                }
                Instruction::EvaluateLazyOperator(op_type, args) => {
                    self.evaluate_lazy_operator(op_type, args, data, arena)?;
                }
            }
        }

        // The final result should be the only value on the stack
        if self.values.len() == 1 {
            Ok(self.values.pop().unwrap())
        } else {
            Err(datavalue_rs::Error::Custom(format!(
                "Invalid evaluation result: {} values on stack",
                self.values.len()
            )))
        }
    }

    /// Processes a token and adds appropriate instructions to the stack
    fn process_token(
        &mut self,
        token: &'a Token<'a>,
        data: &'a DataValue<'a>,
        arena: &'a Bump,
    ) -> Result<()> {
        match token {
            // For literal values, just push them onto the value stack
            Token::Literal(value) => {
                self.values.push(value);
            }

            Token::ArrayLiteral(items) => {
                let count = items.len();

                self.instructions.push(Instruction::CollectArray(count));

                for item in items {
                    self.values.push(item);
                }
            }

            // For operators, handle based on the type and evaluation strategy
            Token::Operator { op_type, args } => {
                // Check the evaluation strategy for this operator
                match op_type.evaluation_strategy() {
                    EvaluationStrategy::Eager => {
                        match &**args {
                            Token::ArrayLiteral(items) => {
                                let values: Vec<&DataValue> = items.iter().collect();
                                let count = values.len();

                                // First push the instruction to apply the operator
                                self.instructions
                                    .push(Instruction::CollectOperatorArgs(*op_type, count));

                                // Then push all values (in reverse since stack is LIFO)
                                for value in values {
                                    self.values.push(value);
                                }
                            }
                            Token::Array(items) => {
                                let count = items.len();

                                // First push the instruction to apply the operator after evaluating all items
                                self.instructions
                                    .push(Instruction::CollectOperatorArgs(*op_type, count));

                                // Then process each item, which could be another operator
                                for item in items.iter().rev() {
                                    self.instructions.push(Instruction::Evaluate(item));
                                }
                            }
                            _ => {
                                self.instructions
                                    .push(Instruction::CollectOperatorArgs(*op_type, 1));

                                // For other argument types, continue with recursive processing
                                self.process_token(args, data, arena)?;
                            }
                        }
                    }
                    EvaluationStrategy::Lazy => {
                        // For lazy operators, push a special instruction that will handle the evaluation
                        self.instructions
                            .push(Instruction::EvaluateLazyOperator(*op_type, args));
                    }
                    EvaluationStrategy::Predicate => {
                        // For predicate operators, we could implement special handling if needed
                        // For now, treat them the same as eager operators
                        match &**args {
                            Token::Array(items) => {
                                let count = items.len();

                                self.instructions
                                    .push(Instruction::CollectOperatorArgs(*op_type, count));

                                for item in items.iter().rev() {
                                    self.instructions.push(Instruction::Evaluate(item));
                                }
                            }
                            _ => {
                                return Err(datavalue_rs::Error::Custom(
                                    "Operator requires an array of arguments".to_string(),
                                ));
                            }
                        }
                    }
                }
            }

            // For arrays, evaluate each item and collect the results
            Token::Array(items) => {
                let count = items.len();

                // Push CollectArray instruction first (to be executed after all items are evaluated)
                self.instructions.push(Instruction::CollectArray(count));

                // Then push each item evaluation instruction in reverse order
                for item in items.iter().rev() {
                    self.instructions.push(Instruction::Evaluate(item));
                }
            }

            // Other token types would be handled here
            Token::Variable {
                path,
                default,
                scope_jump,
            } => {
                self.evaluate_variable(path, default, scope_jump, data, arena)?;
            }

            Token::DynamicVariable {
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
                self.evaluate_variable(path, &evaluated_default, scope_jump, data, arena)?;
            }

            Token::CustomOperator { .. } => {
                return Err(datavalue_rs::Error::Custom(
                    "Custom operators not yet implemented".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Evaluates a lazy operator with its raw arguments
    fn evaluate_lazy_operator(
        &mut self,
        op_type: OperatorType,
        args: &'a Token<'a>,
        data: &'a DataValue<'a>,
        arena: &'a Bump,
    ) -> Result<()> {
        // Call the appropriate lazy operator function
        let result = match op_type {
            // Logic operators
            OperatorType::If => logic::evaluate_if(args, data, arena)?,
            OperatorType::And => logic::evaluate_and(args, data, arena)?,
            OperatorType::Or => logic::evaluate_or(args, data, arena)?,
            OperatorType::Not => logic::evaluate_not(args, data, arena)?,
            OperatorType::DoubleBang => logic::evaluate_double_bang(args, data, arena)?,
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
            OperatorType::Merge => collection::evaluate_merge(args, data, arena)?,
            OperatorType::Cat => collection::evaluate_cat(args, data, arena)?,
            OperatorType::Reduce => collection::evaluate_reduce(args, data, arena)?,

            // For other lazy operators that might be implemented later
            _ => {
                return Err(datavalue_rs::Error::Custom(format!(
                    "Lazy operator {:?} not yet implemented",
                    op_type
                )));
            }
        };

        // Push the result onto the value stack
        self.values.push(result);

        Ok(())
    }

    /// Collects array items from the value stack
    fn collect_array(&mut self, count: usize, arena: &'a Bump) -> Result<()> {
        if self.values.len() < count {
            return Err(datavalue_rs::Error::Custom(format!(
                "Not enough values on stack for array: need {}, have {}",
                count,
                self.values.len()
            )));
        }

        // Get the array items from the stack
        let start = self.values.len() - count;
        let array_items: Vec<_> = self.values.drain(start..).collect();

        // Create the array value
        // Convert to the expected DataValue type
        let array_values: Vec<DataValue<'a>> = array_items.iter().map(|&v| v.clone()).collect();

        let array = DataValue::Array(arena.alloc_slice_fill_iter(array_values));
        self.values.push(arena.alloc(array));

        Ok(())
    }

    /// Collects operator arguments and applies the operator
    fn collect_operator_args(
        &mut self,
        op_type: OperatorType,
        count: usize,
        arena: &'a Bump,
    ) -> Result<()> {
        if self.values.len() < count {
            return Err(datavalue_rs::Error::Custom(format!(
                "Not enough values on stack for operator: need {}, have {}",
                count,
                self.values.len()
            )));
        }

        // Get the operator arguments from the stack
        let start = self.values.len() - count;
        let arg_refs: Vec<_> = self.values.drain(start..).collect();

        // Convert references to DataValue objects
        let args: Vec<DataValue<'a>> = arg_refs.iter().map(|&v| v.clone()).collect();

        // Apply the operator based on its type
        match op_type {
            OperatorType::Add => {
                let result = arithmetic::evaluate_add(&args, arena)?;
                self.values.push(result);
            }
            OperatorType::Subtract => {
                let result = arithmetic::evaluate_subtract(&args, arena)?;
                self.values.push(result);
            }
            OperatorType::Multiply => {
                let result = arithmetic::evaluate_multiply(&args, arena)?;
                self.values.push(result);
            }
            OperatorType::Divide => {
                let result = arithmetic::evaluate_divide(&args, arena)?;
                self.values.push(result);
            }
            OperatorType::Modulo => {
                let result = arithmetic::evaluate_modulo(&args, arena)?;
                self.values.push(result);
            }
            OperatorType::Min => {
                let result = arithmetic::evaluate_min(&args, arena)?;
                self.values.push(result);
            }
            OperatorType::Max => {
                let result = arithmetic::evaluate_max(&args, arena)?;
                self.values.push(result);
            }
            OperatorType::Abs => {
                let result = arithmetic::evaluate_abs(&args, arena)?;
                self.values.push(result);
            }
            OperatorType::Ceil => {
                let result = arithmetic::evaluate_ceil(&args, arena)?;
                self.values.push(result);
            }
            OperatorType::Floor => {
                let result = arithmetic::evaluate_floor(&args, arena)?;
                self.values.push(result);
            }
            OperatorType::Missing => {
                // Pass the arguments directly
                let result = misc::evaluate_missing_args(&args, self.data.unwrap(), arena)?;
                self.values.push(result);
            }
            OperatorType::MissingSome => {
                // Pass the arguments directly
                let result = misc::evaluate_missing_some_args(&args, self.data.unwrap(), arena)?;
                self.values.push(result);
            }
            OperatorType::Exists => {
                // Pass the arguments directly
                let result = misc::evaluate_exists_args(&args, self.data.unwrap(), arena)?;
                self.values.push(result);
            }
            OperatorType::Substring => {
                // Pass the arguments directly
                let result = string::evaluate_substring(&args, arena)?;
                self.values.push(result);
            }
            _ => {
                return Err(datavalue_rs::Error::Custom(format!(
                    "Operator {:?} not yet implemented in stack engine",
                    op_type
                )));
            }
        }

        Ok(())
    }

    fn evaluate_variable(
        &mut self,
        path: &'a DataValue<'a>,
        default: &Option<&'a DataValue<'a>>,
        scope_jump: &Option<usize>,
        data: &'a DataValue<'a>,
        arena: &'a Bump,
    ) -> Result<()> {
        if scope_jump.is_none() {
            match path {
                DataValue::Array([]) | DataValue::Null => {
                    self.values.push(data);
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
                        self.values.push(context);
                    } else if let Some(default) = default {
                        self.values.push(arena.alloc(default));
                    } else {
                        self.values.push(arena.alloc(DataValue::Null));
                    }
                }
                DataValue::String(key) => {
                    let value = data.get(key);
                    if let Some(value) = value {
                        self.values.push(value);
                    } else if let Some(default) = default {
                        self.values.push(arena.alloc(default));
                    } else {
                        self.values.push(arena.alloc(DataValue::Null));
                    }
                }
                DataValue::Number(Number::Integer(i)) => {
                    let value = data.get_index(*i as usize);
                    if let Some(value) = value {
                        self.values.push(value);
                    } else if let Some(default) = default {
                        self.values.push(arena.alloc(default));
                    } else {
                        self.values.push(arena.alloc(DataValue::Null));
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
        &mut self,
        token: &'a Token<'a>,
        data: &'a DataValue<'a>,
        arena: &'a Bump,
    ) -> Result<&'a DataValue<'a>> {
        // Create a temporary instruction stack to evaluate the token
        let mut temp_stack = InstructionStack::new(token);

        // Evaluate the token and return the result
        temp_stack.evaluate(data, arena)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Token;
    use datavalue_rs::helpers;

    #[test]
    fn test_simple_addition() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a simple addition: 1 + 2 + 3
        let arg1 = Box::new(Token::Literal(helpers::int(1)));
        let arg2 = Box::new(Token::Literal(helpers::int(2)));
        let arg3 = Box::new(Token::Literal(helpers::int(3)));

        let args = Box::new(Token::Array(vec![arg1, arg2, arg3]));
        let add_token = Token::Operator {
            op_type: OperatorType::Add,
            args,
        };

        // Evaluate using our stack-based engine
        let mut stack = InstructionStack::new(&add_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(6));
    }

    #[test]
    fn test_nested_addition() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a nested addition: (1 + 2) + (3 + 4)
        let inner_args1 = Box::new(Token::ArrayLiteral(vec![
            DataValue::Number(Number::Integer(1)),
            DataValue::Number(Number::Integer(2)),
        ]));

        let inner_args2 = Box::new(Token::ArrayLiteral(vec![
            DataValue::Number(Number::Integer(3)),
            DataValue::Number(Number::Integer(4)),
        ]));

        let inner_add1 = Box::new(Token::Operator {
            op_type: OperatorType::Add,
            args: inner_args1,
        });

        let inner_add2 = Box::new(Token::Operator {
            op_type: OperatorType::Add,
            args: inner_args2,
        });

        let args = Box::new(Token::Array(vec![inner_add1, inner_add2]));
        let add_token = Token::Operator {
            op_type: OperatorType::Add,
            args,
        };

        // Evaluate using our stack-based engine
        let mut stack = InstructionStack::new(&add_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(10));
    }

    #[test]
    fn test_subtract() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a subtraction: 10 - 3 - 2
        let arg1 = Box::new(Token::Literal(helpers::int(10)));
        let arg2 = Box::new(Token::Literal(helpers::int(3)));
        let arg3 = Box::new(Token::Literal(helpers::int(2)));

        let args = Box::new(Token::Array(vec![arg1, arg2, arg3]));
        let subtract_token = Token::Operator {
            op_type: OperatorType::Subtract,
            args,
        };

        // Evaluate using our stack-based engine
        let mut stack = InstructionStack::new(&subtract_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(5)); // 10 - 3 - 2 = 5
    }

    #[test]
    fn test_multiply() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        let args = Box::new(Token::ArrayLiteral(vec![
            DataValue::Number(Number::Integer(2)),
            DataValue::Number(Number::Integer(3)),
            DataValue::Number(Number::Integer(4)),
        ]));
        let multiply_token = Token::Operator {
            op_type: OperatorType::Multiply,
            args,
        };

        // Evaluate using our stack-based engine
        let mut stack = InstructionStack::new(&multiply_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(24)); // 2 * 3 * 4 = 24
    }

    #[test]
    fn test_divide() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        let args = Box::new(Token::ArrayLiteral(vec![
            DataValue::Number(Number::Integer(20)),
            DataValue::Number(Number::Integer(4)),
            DataValue::Number(Number::Integer(2)),
        ]));
        let divide_token = Token::Operator {
            op_type: OperatorType::Divide,
            args,
        };

        // Evaluate using our stack-based engine
        let mut stack = InstructionStack::new(&divide_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::float(2.5)); // 20 / 4 / 2 = 2.5
    }

    #[test]
    fn test_modulo() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        let args = Box::new(Token::ArrayLiteral(vec![
            DataValue::Number(Number::Integer(17)),
            DataValue::Number(Number::Integer(5)),
        ]));
        let modulo_token = Token::Operator {
            op_type: OperatorType::Modulo,
            args,
        };

        // Evaluate using our stack-based engine
        let mut stack = InstructionStack::new(&modulo_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(2)); // 17 % 5 = 2
    }

    #[test]
    fn test_min() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a custom min operator: min(5, 2, 8, 1, 9)
        let min_token = Token::Operator {
            op_type: OperatorType::Min,
            args: Box::new(Token::ArrayLiteral(vec![
                DataValue::Number(Number::Integer(5)),
                DataValue::Number(Number::Integer(2)),
                DataValue::Number(Number::Integer(8)),
                DataValue::Number(Number::Integer(1)),
                DataValue::Number(Number::Integer(9)),
            ])),
        };

        // Evaluate using our stack-based engine
        let mut stack = InstructionStack::new(&min_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(1)); // min(5, 2, 8, 1, 9) = 1
    }

    #[test]
    fn test_max() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a custom max operator: max(5, 2, 8, 1, 9)
        let max_token = Token::Operator {
            op_type: OperatorType::Max,
            args: Box::new(Token::ArrayLiteral(vec![
                DataValue::Number(Number::Integer(5)),
                DataValue::Number(Number::Integer(2)),
                DataValue::Number(Number::Integer(8)),
                DataValue::Number(Number::Integer(1)),
                DataValue::Number(Number::Integer(9)),
            ])),
        };

        // Evaluate using our stack-based engine
        let mut stack = InstructionStack::new(&max_token);
        let result = stack.evaluate(data, &arena).unwrap();

        // Check the result
        assert_eq!(result, &helpers::int(9)); // max(5, 2, 8, 1, 9) = 9
    }

    #[test]
    fn test_abs() {
        let arena = Bump::new();
        let data = arena.alloc(DataValue::Null);

        // Create a custom abs operator: abs(-5, 2, 8, 1, 9)
        let abs_token = Token::Operator {
            op_type: OperatorType::Abs,
            args: Box::new(Token::ArrayLiteral(vec![
                DataValue::Number(Number::Integer(-5)),
                DataValue::Number(Number::Integer(2)),
                DataValue::Number(Number::Integer(8)),
                DataValue::Number(Number::Integer(1)),
                DataValue::Number(Number::Integer(9)),
            ])),
        };

        // Evaluate using our stack-based engine
        let mut stack = InstructionStack::new(&abs_token);
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
        let ceil_token = Token::Operator {
            op_type: OperatorType::Ceil,
            args: Box::new(Token::ArrayLiteral(vec![
                DataValue::Number(Number::Float(1.2)),
                DataValue::Number(Number::Float(2.7)),
                DataValue::Number(Number::Float(3.8)),
                DataValue::Number(Number::Float(4.3)),
            ])),
        };

        // Evaluate using our stack-based engine
        let mut stack = InstructionStack::new(&ceil_token);
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
        let floor_token = Token::Operator {
            op_type: OperatorType::Floor,
            args: Box::new(Token::ArrayLiteral(vec![
                DataValue::Number(Number::Float(1.2)),
                DataValue::Number(Number::Float(2.7)),
                DataValue::Number(Number::Float(3.8)),
                DataValue::Number(Number::Float(4.3)),
            ])),
        };

        // Evaluate using our stack-based engine
        let mut stack = InstructionStack::new(&floor_token);
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
