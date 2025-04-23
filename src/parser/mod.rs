//! Parser module for DataLogic expressions
//!
//! This module provides the basic building blocks for parsing logic expressions.

use bumpalo::Bump;
use datavalue_rs::DataValue;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

use crate::optimizer;

mod tests;

pub mod jsonlogic;

/// Result type for parser operations
pub type Result<T> = std::result::Result<T, ParserError>;

/// Error types for parser operations
#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Parser error: {reason}")]
    ParserError { reason: String },

    #[error("Operator not found: {operator}")]
    OperatorNotFoundError { operator: String },

    #[error("Invalid operator arguments: {reason}")]
    InvalidOperatorArgumentsError { reason: String },
}

/// The type of operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorType {
    // Arithmetic operators
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
    Modulo,   // %
    Min,      // min
    Max,      // max
    Abs,      // abs
    Ceil,     // ceil
    Floor,    // floor

    // Comparison operators
    Equal,          // ==
    StrictEqual,    // ===
    NotEqual,       // !=
    StrictNotEqual, // !==
    GT,             // >
    GTE,            // >=
    LT,             // <
    LTE,            // <=

    // Logic operators
    If,
    And,
    Or,
    Not,
    NullCoalesce, // ?? (null coalescing)

    // Special operators
    Var,         // Variable access
    Missing,     // Check if variables are missing
    MissingAll,  // Check if all variables are missing
    MissingSome, // Check if some variables are missing
    Val,         // Evaluate a value
    Exists,      // Check if a variable exists
    Map,         // Map an array
    Filter,      // Filter an array
    Reduce,      // Reduce an array
    All,         // Check if all items in an array match a condition
    Some,        // Check if some items in an array match a condition
    None,        // Check if no items in an array match a condition
    Merge,       // Merge arrays
    In,          // Check if a value is in an array
    Cat,         // Concatenate strings
    Log,         // Log a value (for debugging)
    Custom,      // Custom operator
}

/// Determines how an operator's arguments should be evaluated
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationStrategy {
    /// Evaluate all arguments before calling operator function (eager evaluation)
    Eager,

    /// Pass raw arguments to operator function to control evaluation (lazy evaluation)
    Lazy,

    /// Evaluate arguments and use them in a predicate function
    Predicate,
}

impl OperatorType {
    /// Returns the evaluation strategy for this operator
    pub fn evaluation_strategy(&self) -> EvaluationStrategy {
        match self {
            // Control flow operators with lazy evaluation
            OperatorType::If
            | OperatorType::And
            | OperatorType::Or
            | OperatorType::Not
            | OperatorType::NullCoalesce => EvaluationStrategy::Lazy,

            // Variables and data access operations
            OperatorType::Var
            | OperatorType::Missing
            | OperatorType::MissingAll
            | OperatorType::MissingSome
            | OperatorType::Exists => EvaluationStrategy::Lazy,

            // Array operations that might need special evaluation
            OperatorType::Map
            | OperatorType::Filter
            | OperatorType::Reduce
            | OperatorType::All
            | OperatorType::Some
            | OperatorType::None => EvaluationStrategy::Lazy,

            // Comparison operators
            OperatorType::Equal
            | OperatorType::StrictEqual
            | OperatorType::NotEqual
            | OperatorType::StrictNotEqual
            | OperatorType::GT
            | OperatorType::GTE
            | OperatorType::LT
            | OperatorType::LTE => EvaluationStrategy::Lazy,

            // Default for arithmetic and most other operators - eager evaluation
            _ => EvaluationStrategy::Eager,
        }
    }
}

impl FromStr for OperatorType {
    type Err = ParserError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            // Arithmetic operators
            "+" => Ok(OperatorType::Add),
            "-" => Ok(OperatorType::Subtract),
            "*" => Ok(OperatorType::Multiply),
            "/" => Ok(OperatorType::Divide),
            "%" => Ok(OperatorType::Modulo),
            "min" => Ok(OperatorType::Min),
            "max" => Ok(OperatorType::Max),
            "abs" => Ok(OperatorType::Abs),
            "ceil" => Ok(OperatorType::Ceil),
            "floor" => Ok(OperatorType::Floor),

            // Comparison operators
            "==" => Ok(OperatorType::Equal),
            "===" => Ok(OperatorType::StrictEqual),
            "!=" => Ok(OperatorType::NotEqual),
            "!==" => Ok(OperatorType::StrictNotEqual),
            ">" => Ok(OperatorType::GT),
            ">=" => Ok(OperatorType::GTE),
            "<" => Ok(OperatorType::LT),
            "<=" => Ok(OperatorType::LTE),

            // Logic operators
            "if" => Ok(OperatorType::If),
            "and" => Ok(OperatorType::And),
            "or" => Ok(OperatorType::Or),
            "not" => Ok(OperatorType::Not),
            "??" => Ok(OperatorType::NullCoalesce),

            // Special operators
            "var" => Ok(OperatorType::Var),
            "missing" => Ok(OperatorType::Missing),
            "missing_all" => Ok(OperatorType::MissingAll),
            "missing_some" => Ok(OperatorType::MissingSome),
            "val" => Ok(OperatorType::Val),
            "exists" => Ok(OperatorType::Exists),
            "map" => Ok(OperatorType::Map),
            "filter" => Ok(OperatorType::Filter),
            "reduce" => Ok(OperatorType::Reduce),
            "all" => Ok(OperatorType::All),
            "some" => Ok(OperatorType::Some),
            "none" => Ok(OperatorType::None),
            "merge" => Ok(OperatorType::Merge),
            "in" => Ok(OperatorType::In),
            "cat" => Ok(OperatorType::Cat),
            "log" => Ok(OperatorType::Log),
            _ => Err(ParserError::OperatorNotFoundError {
                operator: s.to_string(),
            }),
        }
    }
}

impl fmt::Display for OperatorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            // Arithmetic operators
            OperatorType::Add => "+",
            OperatorType::Subtract => "-",
            OperatorType::Multiply => "*",
            OperatorType::Divide => "/",
            OperatorType::Modulo => "%",
            OperatorType::Min => "min",
            OperatorType::Max => "max",
            OperatorType::Abs => "abs",
            OperatorType::Ceil => "ceil",
            OperatorType::Floor => "floor",

            // Logic operators
            OperatorType::If => "if",
            OperatorType::And => "and",
            OperatorType::Or => "or",
            OperatorType::Not => "not",
            OperatorType::NullCoalesce => "??",

            // Comparison operators
            OperatorType::Equal => "==",
            OperatorType::StrictEqual => "===",
            OperatorType::NotEqual => "!=",
            OperatorType::StrictNotEqual => "!==",
            OperatorType::GT => ">",
            OperatorType::GTE => ">=",
            OperatorType::LT => "<",
            OperatorType::LTE => "<=",

            // Special operators
            OperatorType::Var => "var",
            OperatorType::Missing => "missing",
            OperatorType::MissingAll => "missing_all",
            OperatorType::MissingSome => "missing_some",
            OperatorType::Val => "val",
            OperatorType::Exists => "exists",
            OperatorType::Map => "map",
            OperatorType::Filter => "filter",
            OperatorType::Reduce => "reduce",
            OperatorType::All => "all",
            OperatorType::Some => "some",
            OperatorType::None => "none",
            OperatorType::Merge => "merge",
            OperatorType::In => "in",
            OperatorType::Cat => "cat",
            OperatorType::Log => "log",
            OperatorType::Custom => "custom",
        };
        write!(f, "{}", name)
    }
}

/// Token representing an expression component
#[derive(Debug, PartialEq, Clone)]
pub enum Token<'a> {
    /// A literal value
    Literal(DataValue<'a>),

    /// An array of tokens
    Array(Vec<Box<Token<'a>>>),

    /// An array of literal values
    ArrayLiteral(Vec<DataValue<'a>>),

    /// A variable reference with an optional default value
    Variable {
        path: &'a DataValue<'a>,
        default: Option<&'a DataValue<'a>>,
        scope_jump: Option<usize>,
    },

    /// A dynamic variable reference where the path is computed at runtime
    DynamicVariable {
        path_expr: Box<Token<'a>>,
        default: Option<Box<Token<'a>>>,
        scope_jump: Option<usize>,
    },

    /// An operator with a name and arguments
    Operator {
        op_type: OperatorType,
        args: Box<Token<'a>>,
    },

    /// A custom operator with a name and arguments
    CustomOperator { name: &'a str, args: Box<Token<'a>> },
}

impl<'a> Token<'a> {
    /// Create a new custom operator token
    pub fn custom_operator(name: &'a str, args: Box<Token<'a>>) -> Self {
        Token::CustomOperator { name, args }
    }

    pub fn is_operator(&self) -> bool {
        matches!(self, Token::Operator { .. })
    }

    pub fn is_literal(&self) -> bool {
        matches!(self, Token::Literal(_))
    }

    pub fn is_variable(&self) -> bool {
        matches!(self, Token::Variable { .. })
    }

    pub fn is_dynamic_variable(&self) -> bool {
        matches!(self, Token::DynamicVariable { .. })
    }

    pub fn is_custom_operator(&self) -> bool {
        matches!(self, Token::CustomOperator { .. })
    }

    pub fn is_static_token(&self) -> bool {
        // Static tokens are those that don't require runtime evaluation
        match self {
            // Literals are always static
            Token::Literal(_) => true,

            // Array literals are static if all their elements are static
            Token::ArrayLiteral(_) => true,

            // Arrays are never static as they can contain dynamic variables
            Token::Array(items) => items.iter().all(|item| item.is_static_token()),

            // Variables are never static as they access the context
            Token::Variable { .. } => false,
            Token::DynamicVariable { .. } => false,

            // Operators are static if they don't access variables and all their arguments are static
            Token::Operator { op_type, args } => {
                // These operators access variables directly
                match op_type {
                    OperatorType::Var
                    | OperatorType::Missing
                    | OperatorType::MissingAll
                    | OperatorType::MissingSome
                    | OperatorType::Exists => false,

                    // For other operators, check if their arguments are static
                    _ => args.is_static_token(),
                }
            }

            // Custom operators are considered non-static by default
            Token::CustomOperator { .. } => false,
        }
    }
}

/// Parse a JSON string into a JSONLogic Token
pub fn parser<'a>(input: &str, arena: &'a Bump) -> Result<&'a Token<'a>> {
    let data_value = DataValue::from_str(arena, input).map_err(|e| ParserError::ParserError {
        reason: format!("Invalid JSON: {}", e),
    })?;

    // Parse the DataValue
    parser_value(&data_value, arena)
}

/// Parse a DataValue into a JSONLogic Token
pub fn parser_value<'a>(input: &DataValue<'a>, arena: &'a Bump) -> Result<&'a Token<'a>> {
    let token = jsonlogic::parse_datavalue_internal(input, arena)?;
    Ok(arena.alloc(token))
}

pub fn optimize_token<'a>(token: &'a Token<'a>, arena: &'a Bump) -> Result<Box<Token<'a>>> {
    let optimized = optimizer::optimize(token, arena);
    Ok(optimized)
}
