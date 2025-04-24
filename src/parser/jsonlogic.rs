//! JSONLogic parser implementation
//!
//! This module provides the parser for JSONLogic expressions using DataValue.

use std::str::FromStr;

use bumpalo::Bump;
use datavalue_rs::{helpers, DataValue, Number};

use crate::parser::{OperatorType, ParserError, Result, Token};

/// Internal function for parsing a DataValue into a token.
pub fn parse_datavalue_internal<'a>(value: &DataValue<'a>, arena: &'a Bump) -> Result<Token<'a>> {
    match value {
        // Simple literals
        DataValue::Null => Ok(Token::Literal(helpers::null())),
        DataValue::Bool(b) => Ok(Token::Literal(helpers::boolean(*b))),
        DataValue::Number(Number::Integer(i)) => Ok(Token::Literal(helpers::int(*i))),
        DataValue::Number(Number::Float(f)) => Ok(Token::Literal(helpers::float(*f))),
        DataValue::String(s) => Ok(Token::Literal(helpers::string(arena, s))),
        DataValue::DateTime(dt) => Ok(Token::Literal(DataValue::DateTime(*dt))),
        DataValue::Duration(d) => Ok(Token::Literal(DataValue::Duration(*d))),

        // Arrays could be literal arrays or token arrays
        DataValue::Array(arr) => parse_array(arr, arena),

        // Objects could be operators or literal objects
        DataValue::Object(entries) => parse_object(entries, arena),
    }
}

fn parse_array<'a>(arr: &[DataValue<'a>], arena: &'a Bump) -> Result<Token<'a>> {
    let mut values = Vec::with_capacity(arr.len());
    let mut tokens = Vec::with_capacity(arr.len());
    for item in arr.iter() {
        let token = parse_datavalue_internal(item, arena)?;
        if matches!(token, Token::Literal(_)) {
            values.push(item.clone());
            tokens.push(Box::new(token));
        } else {
            tokens.push(Box::new(token));
        }
    }

    if values.len() == arr.len() {
        Ok(Token::ArrayLiteral(values))
    } else {
        Ok(Token::Array(tokens))
    }
}

/// Parses a JSON object into a token.
fn parse_object<'a>(entries: &'a [(&'a str, DataValue<'a>)], arena: &'a Bump) -> Result<Token<'a>> {
    // If the object has exactly one key, it might be an operator
    if entries.len() == 1 {
        let (key, value) = &entries[0];

        match *key {
            "var" => parse_variable(value, arena),
            "val" => parse_val(value, arena),
            "preserve" => {
                // The preserve operator returns its argument as-is without parsing it as an operator
                Ok(Token::Literal(value.clone()))
            }
            _ => {
                // Check if it's a standard operator
                if let Ok(op_type) = OperatorType::from_str(key) {
                    parse_operator(op_type, value, arena)
                } else {
                    // Otherwise, treat it as a custom operator
                    parse_custom_operator(key, value, arena)
                }
            }
        }
    } else if entries.is_empty() {
        // Empty object literal
        Ok(Token::Literal(helpers::object(arena, vec![])))
    } else {
        // For multi-key objects, treat the first key as an unknown operator
        // This matches the JSONLogic behavior where multi-key objects should
        // fail as unknown operators rather than parse errors
        let (key, _) = &entries[0];

        // Return an OperatorNotFoundError instead of a ParseError
        Err(ParserError::OperatorNotFoundError {
            operator: key.to_string(),
        })
    }
}

fn parse_val<'a>(value: &DataValue<'a>, arena: &'a Bump) -> Result<Token<'a>> {
    match value {
        DataValue::String(s) => Ok(Token::Variable {
            path: arena.alloc(DataValue::String(s)),
            default: None,
            scope_jump: None,
        }),
        DataValue::Number(Number::Integer(i)) => Ok(Token::Variable {
            path: arena.alloc(DataValue::Number(Number::Integer(*i))),
            default: None,
            scope_jump: None,
        }),
        DataValue::Array(arr) => {
            if arr.len() > 1 {
                if let DataValue::Array([DataValue::Number(Number::Integer(i))]) = arr[0] {
                    let jump = *i as usize;
                    return Ok(Token::DynamicVariable {
                        path_expr: Box::new(Token::Literal(DataValue::Array(&arr[1..]))),
                        default: None,
                        scope_jump: Some(jump),
                    });
                }
            }

            let is_dynamic = arr.iter().any(|item| matches!(item, DataValue::Object(_)));

            if is_dynamic {
                Ok(Token::DynamicVariable {
                    path_expr: Box::new(Token::Literal(DataValue::Array(arr))),
                    default: None,
                    scope_jump: None,
                })
            } else {
                Ok(Token::Variable {
                    path: arena.alloc(DataValue::Array(arr)),
                    default: None,
                    scope_jump: None,
                })
            }
        }
        DataValue::Object(entries) => {
            let operator = parse_object(entries, arena)?;
            Ok(Token::DynamicVariable {
                path_expr: Box::new(operator),
                default: None,
                scope_jump: None,
            })
        }
        _ => Err(ParserError::ParserError {
            reason: format!("Invalid val syntax: {:?}", value),
        }),
    }
}

fn parse_variable_path<'a>(path: &'a str, arena: &'a Bump) -> Result<&'a DataValue<'a>> {
    if path.contains('.') {
        let parts: Vec<&str> = path.split('.').collect();
        let parts_data_values: Vec<DataValue> = parts
            .iter()
            .map(|p| {
                if let Ok(number) = p.parse::<i64>() {
                    DataValue::Number(Number::Integer(number))
                } else {
                    DataValue::String(p)
                }
            })
            .collect();
        let path_array = arena.alloc(DataValue::Array(
            arena.alloc_slice_fill_iter(parts_data_values),
        ));
        return Ok(path_array);
    }

    if path.is_empty() {
        return Ok(arena.alloc(DataValue::Array(&[])));
    }

    if let Ok(number) = path.parse::<i64>() {
        return Ok(arena.alloc(DataValue::Number(Number::Integer(number))));
    }

    Ok(arena.alloc(DataValue::String(path)))
}

/// Parses a variable reference.
fn parse_variable<'a>(var_value: &DataValue<'a>, arena: &'a Bump) -> Result<Token<'a>> {
    match var_value {
        // Simple variable reference
        DataValue::String(path) => {
            let value = parse_variable_path(path, arena)?;
            Ok(Token::Variable {
                path: value,
                default: None,
                scope_jump: None,
            })
        }

        DataValue::Number(Number::Integer(i)) => Ok(Token::Variable {
            path: arena.alloc(DataValue::Number(Number::Integer(*i))),
            default: None,
            scope_jump: None,
        }),

        DataValue::Null => {
            let value = arena.alloc(DataValue::Array(&[]));
            Ok(Token::Variable {
                path: value,
                default: None,
                scope_jump: None,
            })
        }

        // Variable reference with default value
        DataValue::Array(arr) => {
            // Handle empty array - treat it as a reference to the data itself
            if arr.is_empty() {
                let path_data_value = arena.alloc(DataValue::Array(arr));
                return Ok(Token::Variable {
                    path: path_data_value,
                    default: None,
                    scope_jump: None,
                });
            }

            // Get the path (first element)
            let path = &arr[0];
            // If there's a default value, parse it
            let default = if arr.len() >= 2 { Some(&arr[1]) } else { None };

            // For complex expressions in the path, we need to create a special token
            // that will evaluate the path at runtime
            if matches!(path, DataValue::Object(_))
                || matches!(default, Some(&DataValue::Object(_)))
            {
                // Parse the path expression
                let path_expr = parse_datavalue_internal(path, arena)?;
                let default_expr = if let Some(default) = default {
                    Some(Box::new(parse_datavalue_internal(default, arena)?))
                } else {
                    None
                };
                return Ok(Token::DynamicVariable {
                    path_expr: Box::new(path_expr),
                    default: default_expr,
                    scope_jump: None,
                });
            }

            Ok(Token::Variable {
                path,
                default,
                scope_jump: None,
            })
        }

        // Anything else is an error
        _ => Err(ParserError::ParserError {
            reason: format!("Invalid variable syntax: {:?}", var_value),
        }),
    }
}

/// Parse an operator and its arguments
fn parse_operator<'a>(
    op_type: OperatorType,
    args_value: &DataValue<'a>,
    _arena: &'a Bump,
) -> Result<Token<'a>> {
    Ok(Token::Operator {
        op_type,
        args: Box::new(parse_datavalue_internal(args_value, _arena)?),
    })
}

/// Parse a custom operator and its arguments
fn parse_custom_operator<'a>(
    name: &str,
    args_value: &DataValue<'a>,
    arena: &'a Bump,
) -> Result<Token<'a>> {
    Ok(Token::CustomOperator {
        name: arena.alloc_str(name),
        args: Box::new(parse_datavalue_internal(args_value, arena)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use datavalue_rs::Bump;

    #[test]
    fn test_parse_literals() {
        let arena = Bump::new();

        // Null
        let null_json = r#"null"#;
        let token = parser::parser(null_json, &arena).unwrap();
        match token {
            Token::Literal(DataValue::Null) => (),
            _ => panic!("Expected null literal"),
        }

        // Boolean
        let bool_json = r#"true"#;
        let token = parser::parser(bool_json, &arena).unwrap();
        match token {
            Token::Literal(DataValue::Bool(true)) => (),
            _ => panic!("Expected boolean literal"),
        }

        // Integer
        let int_json = r#"42"#;
        let token = parser::parser(int_json, &arena).unwrap();
        match token {
            Token::Literal(DataValue::Number(Number::Integer(i))) => assert_eq!(*i, 42),
            _ => panic!("Expected integer literal"),
        }

        // Float
        let float_json = r#"3.14"#;
        let token = parser::parser(float_json, &arena).unwrap();
        match token {
            Token::Literal(DataValue::Number(Number::Float(f))) => {
                assert!((*f - 3.14).abs() < f64::EPSILON)
            }
            _ => panic!("Expected float literal"),
        }

        // String
        let string_json = r#""hello""#;
        let token = parser::parser(string_json, &arena).unwrap();
        match token {
            Token::Literal(DataValue::String(s)) => assert_eq!(*s, "hello"),
            _ => panic!("Expected string literal"),
        }

        // Array of literals
        let array_json = r#"[1, 2, 3]"#;
        let token = parser::parser(array_json, &arena).unwrap();
        println!("{:?}", token);
        match token {
            Token::ArrayLiteral(arr) => {
                assert_eq!(arr.len(), 3);
                match arr[0] {
                    DataValue::Number(Number::Integer(i)) => assert_eq!(i, 1),
                    _ => panic!("Expected integer as first array element"),
                }
            }
            _ => panic!("Expected array literal"),
        }
    }

    #[test]
    fn test_parse_variable() {
        let arena = Bump::new();

        // Simple variable
        let var_json = r#"{"var": "user.name"}"#;
        let token = parser::parser(var_json, &arena).unwrap();
        match token {
            Token::Variable {
                path,
                default,
                scope_jump: _,
            } => {
                // Handle both string and array paths (for dotted notation)
                match path {
                    DataValue::String(s) => assert_eq!(*s, "user.name"),
                    DataValue::Array(arr) => {
                        // For dotted path, it's stored as an array of strings
                        if !arr.is_empty() {
                            let mut path_parts = Vec::new();
                            for part in arr.iter() {
                                if let DataValue::String(s) = part {
                                    path_parts.push(*s);
                                }
                            }
                            if path_parts.len() == arr.len() {
                                assert_eq!(path_parts.join("."), "user.name");
                                return;
                            }
                        }
                        panic!("Unexpected array path format");
                    }
                    _ => panic!("Expected string or array path"),
                }
                assert!(default.is_none());
            }
            _ => panic!("Expected variable token"),
        }

        // Variable with default
        let var_with_default_json = r#"{"var": ["user.name", "Anonymous"]}"#;
        let token = parser::parser(var_with_default_json, &arena).unwrap();
        match token {
            Token::Variable {
                path,
                ref default,
                scope_jump: _,
            } => {
                // Handle both string and array paths
                match path {
                    DataValue::String(s) => assert_eq!(*s, "user.name"),
                    DataValue::Array(arr) => {
                        // For dotted path, it's stored as an array of strings
                        if !arr.is_empty() {
                            let mut path_parts = Vec::new();
                            for part in arr.iter() {
                                if let DataValue::String(s) = part {
                                    path_parts.push(*s);
                                }
                            }
                            if path_parts.len() == arr.len() {
                                assert_eq!(path_parts.join("."), "user.name");
                            } else {
                                panic!("Unexpected array path format");
                            }
                        }
                    }
                    _ => panic!("Expected string or array path"),
                }
                assert!(default.is_some());
                match **default.as_ref().unwrap() {
                    DataValue::String(s) => assert_eq!(s, "Anonymous"),
                    _ => panic!("Expected string literal as default"),
                }
            }
            _ => panic!("Expected variable token with default"),
        }

        // Empty path (reference to data itself)
        let empty_var_json = r#"{"var": []}"#;
        let token = parser::parser(empty_var_json, &arena).unwrap();
        match token {
            Token::Variable {
                path,
                default,
                scope_jump: _,
            } => {
                match path {
                    DataValue::Array(arr) => assert!(arr.is_empty()),
                    _ => panic!("Expected array path"),
                }
                assert!(default.is_none());
            }
            _ => panic!("Expected variable token with empty path"),
        }
    }

    #[test]
    fn test_parse_operator() {
        let arena = Bump::new();

        // Simple operator with single argument
        let op_json = r#"{"!": true}"#;
        let token = parser::parser(op_json, &arena).unwrap();
        match *token {
            Token::Operator { op_type, ref args } => {
                assert_eq!(op_type, OperatorType::Not);
                match **args {
                    Token::Literal(DataValue::Bool(b)) => assert!(b),
                    _ => panic!("Expected boolean argument"),
                }
            }
            _ => panic!("Expected operator token"),
        }

        // Test reduce operator
        let reduce_json = r#"{"reduce": [
            [1, 2, 3, 4],
            {"var": "accumulator"}
        ]}"#;
        let token = parser::parser(reduce_json, &arena).unwrap();
        match *token {
            Token::Operator { op_type, .. } => {
                assert_eq!(op_type, OperatorType::Reduce);
                // No need to validate the exact args structure
            }
            _ => panic!("Expected operator token for reduce"),
        }

        // Operator with multiple arguments
        let op_with_args_json = r#"{"and": [true, false, true]}"#;
        let token = parser::parser(op_with_args_json, &arena).unwrap();
        match *token {
            Token::Operator { op_type, ref args } => {
                assert_eq!(op_type, OperatorType::And);
                match &**args {
                    Token::ArrayLiteral(arr) => {
                        assert_eq!(arr.len(), 3);
                        assert!(matches!(arr[0], DataValue::Bool(true)));
                        assert!(matches!(arr[1], DataValue::Bool(false)));
                        assert!(matches!(arr[2], DataValue::Bool(true)));
                    }
                    _ => panic!("Expected array of arguments"),
                }
            }
            _ => panic!("Expected operator token with arguments"),
        }
    }

    #[test]
    fn test_parse_complex_expression() {
        let arena = Bump::new();

        // Complex expression with nested operators
        let complex_json = r#"
        {
            "if": [
                {"<": [{"var": "temp"}, 0]},
                "freezing",
                {"<": [{"var": "temp"}, 25]},
                "cool",
                "hot"
            ]
        }
        "#;
        let token = parser::parser(complex_json, &arena).unwrap();

        // Just verify it parses without error and has the right structure
        match *token {
            Token::Operator { op_type, .. } => {
                assert_eq!(op_type, OperatorType::If);
            }
            _ => panic!("Expected if operator token"),
        }
    }

    #[test]
    fn test_custom_operator() {
        let arena = Bump::new();

        // JSONLogic expression with a custom operator
        let input = r#"{"custom_op": [1, 2, 3]}"#;

        let token = parser::parser(input, &arena).unwrap();

        // Verify the token is a custom operator
        match *token {
            Token::CustomOperator { name, ref args } => {
                assert_eq!(name, "custom_op");
                match &**args {
                    Token::ArrayLiteral(arr) => {
                        assert_eq!(arr.len(), 3);
                        assert!(matches!(arr[0], DataValue::Number(Number::Integer(1))));
                        assert!(matches!(arr[1], DataValue::Number(Number::Integer(2))));
                        assert!(matches!(arr[2], DataValue::Number(Number::Integer(3))));
                    }
                    _ => panic!("Expected array of arguments"),
                }
            }
            _ => panic!("Expected custom operator token"),
        }
    }
}
