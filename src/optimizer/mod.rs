//! Token optimizer for JSONLogic expressions
//!
//! This module provides functionality to optimize token trees by flattening nested operations
//! and performing constant folding where possible.

use crate::{
    evaluate,
    parser::{OperatorType, Token},
};
use bumpalo::Bump;
use datavalue_rs::DataValue;

mod tests;

/// Create a deep clone of a token
///
/// Since Token doesn't implement Clone, we need to manually create a copy
fn deep_clone<'a>(token: &Token<'a>) -> Token<'a> {
    match token {
        Token::Literal(value) => Token::literal(value.clone()),
        Token::ArrayLiteral(tokens) => {
            let cloned_tokens = tokens.iter().map(|t| Box::new(deep_clone(t))).collect();
            Token::ArrayLiteral(cloned_tokens)
        }
        Token::Variable {
            path,
            default,
            scope_jump,
        } => Token::variable(path, *default, *scope_jump),
        Token::DynamicVariable {
            path_expr,
            default,
            scope_jump,
        } => {
            let cloned_path = Box::new(deep_clone(path_expr));
            let cloned_default = default.as_ref().map(|d| Box::new(deep_clone(d)));
            Token::dynamic_variable(cloned_path, cloned_default, *scope_jump)
        }
        Token::Operator { op_type, args } => {
            let cloned_args = Box::new(deep_clone(args));
            Token::operator(*op_type, cloned_args)
        }
        Token::CustomOperator { name, args } => {
            let cloned_args = Box::new(deep_clone(args));
            Token::custom_operator(name, cloned_args)
        }
    }
}

/// Optimizes a token tree by applying various optimization rules
///
/// # Arguments
///
/// * `token` - The token to optimize
/// * `arena` - The arena allocator for creating new tokens
///
/// # Returns
///
/// A new, optimized token
pub fn optimize<'a>(token: &'a Token<'a>, arena: &'a Bump) -> Box<Token<'a>> {
    if token.is_static_token() {
        // For static tokens, we can try to evaluate them right away and return a literal
        if let Ok(result) = evaluate(token, &DataValue::Null, arena) {
            return Box::new(Token::literal(result.clone()));
        }
    }

    match token {
        Token::Literal(_) => Box::new(deep_clone(token)),
        Token::Variable { .. } => Box::new(deep_clone(token)),
        Token::DynamicVariable { .. } => Box::new(deep_clone(token)),

        Token::ArrayLiteral(tokens) => {
            // Optimize each token in the array
            let optimized_tokens: Vec<Box<Token<'a>>> =
                tokens.iter().map(|t| optimize(t, arena)).collect();

            Box::new(Token::ArrayLiteral(optimized_tokens))
        }

        Token::Operator { op_type, args } => {
            // First, optimize the arguments
            let optimized_args = optimize(args, arena);

            // Simple solution: clone to avoid borrowing issues
            let cloned_args = deep_clone(optimized_args.as_ref());

            // Apply operator-specific optimizations
            match op_type {
                // Operators that can be flattened (Subtract, Modulo and Divide cannot be flattened)
                OperatorType::Add
                | OperatorType::Multiply
                | OperatorType::And
                | OperatorType::Or
                | OperatorType::Abs
                | OperatorType::Ceil
                | OperatorType::Floor => {
                    if let Token::ArrayLiteral(ref tokens) = cloned_args {
                        let mut flattened_tokens = Vec::new();

                        for token_box in tokens {
                            let token = token_box.as_ref();

                            if let Token::Operator {
                                op_type: inner_op,
                                args: inner_args,
                            } = token
                            {
                                if inner_op == op_type {
                                    if let Token::ArrayLiteral(inner_tokens) = inner_args.as_ref() {
                                        // Add all tokens from the inner array
                                        for inner_token in inner_tokens {
                                            let cloned = deep_clone(inner_token.as_ref());
                                            flattened_tokens.push(Box::new(cloned));
                                        }
                                        continue;
                                    } else if let Token::Literal(DataValue::Array(inner_values)) =
                                        inner_args.as_ref()
                                    {
                                        // Add all values from the inner array
                                        for value in inner_values.iter() {
                                            flattened_tokens
                                                .push(Box::new(Token::literal(value.clone())));
                                        }
                                        continue;
                                    }
                                }
                            }

                            // Not a matching operator or not an array, add as is
                            flattened_tokens.push(Box::new(deep_clone(token)));
                        }

                        Box::new(Token::Operator {
                            op_type: *op_type,
                            args: Box::new(Token::ArrayLiteral(flattened_tokens)),
                        })
                    } else {
                        Box::new(Token::Operator {
                            op_type: *op_type,
                            args: Box::new(cloned_args),
                        })
                    }
                }
                // Other operators - just use the optimized args
                _ => Box::new(Token::Operator {
                    op_type: *op_type,
                    args: Box::new(cloned_args),
                }),
            }
        }

        Token::CustomOperator { name, args } => {
            let optimized_args = optimize(args, arena);
            let cloned_args = deep_clone(optimized_args.as_ref());
            Box::new(Token::CustomOperator {
                name,
                args: Box::new(cloned_args),
            })
        }
    }
}
