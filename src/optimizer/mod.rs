//! Token optimizer for JSONLogic expressions
//!
//! This module provides functionality to optimize token trees by flattening nested operations
//! and performing constant folding where possible.

use crate::{evaluate, parser::Token};
use bumpalo::Bump;
use datavalue_rs::DataValue;

/// Create a deep clone of a token
///
/// Since Token doesn't implement Clone, we need to manually create a copy
fn deep_clone<'a>(token: &Token<'a>) -> Token<'a> {
    match token {
        Token::Literal(value) => Token::Literal(value.clone()),
        Token::Array(tokens) => {
            let cloned_tokens = tokens.iter().map(|t| Box::new(deep_clone(t))).collect();
            Token::Array(cloned_tokens)
        }
        Token::ArrayLiteral(values) => {
            let cloned_values = values.to_vec();
            Token::ArrayLiteral(cloned_values)
        }
        Token::Variable {
            path,
            default,
            scope_jump,
        } => Token::Variable {
            path,
            default: *default,
            scope_jump: *scope_jump,
        },
        Token::DynamicVariable {
            path_expr,
            default,
            scope_jump,
        } => Token::DynamicVariable {
            path_expr: Box::new(deep_clone(path_expr)),
            default: default.as_ref().map(|d| Box::new(deep_clone(d))),
            scope_jump: *scope_jump,
        },
        Token::Operator { op_type, args } => Token::Operator {
            op_type: *op_type,
            args: Box::new(deep_clone(args)),
        },
        Token::CustomOperator { name, args } => Token::CustomOperator {
            name,
            args: Box::new(deep_clone(args)),
        },
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
            return Box::new(Token::Literal(result.clone()));
        }
    }

    Box::new(deep_clone(token))
}
