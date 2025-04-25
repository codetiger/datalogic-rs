//! Tests for the parser module
//!
//! This module contains tests for the JSONLogic parser functionality.

#[cfg(test)]
mod tests {
    use crate::parser::{parser, ASTNode, OperatorType, ParserError};
    use bumpalo::Bump;
    use datavalue_rs::{helpers, DataValue, Number};

    #[test]
    fn test_lexer_basic() {
        // Parse simple JSONLogic expression
        let arena = Bump::new();
        let json_str = r#"{"==": [{"var": "a"}, 42]}"#;

        // Parse with the lexer
        let token = parser(json_str, &arena).unwrap();

        // Verify the token structure
        assert!(matches!(token, ASTNode::Operator { .. }));
        match token {
            ASTNode::Operator { op_type, args: _ } => {
                assert_eq!(*op_type, OperatorType::Equal);
            }
            _ => panic!("Expected operator token"),
        }
    }

    #[test]
    fn test_invalid_json() {
        let arena = Bump::new();
        let json_str = r#"{"==": [{"var": "a"}, 42"#; // Missing closing bracket

        // Parse with default parser - should fail
        let result = parser(json_str, &arena);
        assert!(result.is_err());
        match result {
            Err(ParserError::ParserError { reason }) => {
                assert!(reason.contains("Invalid JSON"));
            }
            _ => panic!("Expected ParserError with Invalid JSON message"),
        }
    }

    #[test]
    fn test_lexer_boolean_literal() {
        let arena = Bump::new();
        let json_str = "true";

        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::Literal(DataValue::Bool(b)) => {
                assert!(*b);
            }
            _ => panic!("Expected boolean literal token"),
        }
    }

    #[test]
    fn test_lexer_number_literal() {
        let arena = Bump::new();
        let json_str = "42";

        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::Literal(DataValue::Number(Number::Integer(i))) => {
                assert_eq!(*i, 42);
            }
            _ => panic!("Expected integer literal token"),
        }

        let json_str = "3.14";
        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::Literal(DataValue::Number(Number::Float(f))) => {
                assert!(f > &3.13 && f < &3.15);
            }
            _ => panic!("Expected float literal token"),
        }
    }

    #[test]
    fn test_lexer_string_literal() {
        let arena = Bump::new();
        let json_str = r#""hello world""#;

        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::Literal(DataValue::String(s)) => {
                assert_eq!(*s, "hello world");
            }
            _ => panic!("Expected string literal token"),
        }
    }

    #[test]
    fn test_lexer_array_literal() {
        let arena = Bump::new();
        let json_str = r#"[1, 2, 3]"#;

        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::ArrayLiteral(values) => {
                assert_eq!(values.len(), 3);
                assert!(matches!(values[0], DataValue::Number(Number::Integer(1))));
                assert!(matches!(values[1], DataValue::Number(Number::Integer(2))));
                assert!(matches!(values[2], DataValue::Number(Number::Integer(3))));
            }
            _ => panic!("Expected array literal token"),
        }
    }

    #[test]
    fn test_lexer_object_literal() {
        let arena = Bump::new();
        // Use a preserved object to test literal objects
        let json_str = r#"{"preserve": {"name": "John", "age": 30}}"#;

        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::Literal(DataValue::Object(entries)) => {
                assert_eq!(entries.len(), 2);

                // Find and check the name entry
                let name_entry = entries.iter().find(|(k, _)| *k == "name");
                assert!(name_entry.is_some());
                assert_eq!(name_entry.unwrap().1.as_str().unwrap(), "John");

                // Find and check the age entry
                let age_entry = entries.iter().find(|(k, _)| *k == "age");
                assert!(age_entry.is_some());
                assert_eq!(age_entry.unwrap().1.as_i64().unwrap(), 30);
            }
            _ => panic!("Expected object literal token"),
        }
    }

    #[test]
    fn test_lexer_variable() {
        let arena = Bump::new();
        let json_str = r#"{"var": "user.name"}"#;

        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::Variable {
                path,
                default,
                scope_jump: _,
            } => {
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
                assert!(default.is_none());
            }
            _ => panic!("Expected variable token"),
        }

        // Variable with default value
        let json_str = r#"{"var": ["user.name", "Unknown"]}"#;
        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::Variable {
                path,
                default,
                scope_jump: _,
            } => {
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
                let default_value = default.as_ref().unwrap();
                match **default_value {
                    DataValue::String(s) => assert_eq!(s, "Unknown"),
                    _ => panic!("Expected string literal as default value"),
                }
            }
            _ => panic!("Expected variable token with default"),
        }
    }

    #[test]
    fn test_lexer_operator() {
        let arena = Bump::new();

        // Simple operator
        let json_str = r#"{"!": true}"#;
        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::Operator { op_type, args } => {
                assert_eq!(*op_type, OperatorType::Not);
                match &**args {
                    ASTNode::Literal(DataValue::Bool(b)) => {
                        assert!(*b);
                    }
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
        let token = parser(reduce_json, &arena).unwrap();
        match token {
            ASTNode::Operator { op_type, .. } => {
                assert_eq!(*op_type, OperatorType::Reduce);
                // No need to validate the exact args structure
            }
            _ => panic!("Expected operator token for reduce"),
        }

        // Operator with array argument
        let json_str = r#"{"and": [true, false, true]}"#;
        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::Operator { op_type, args } => {
                assert_eq!(*op_type, OperatorType::And);
                match &**args {
                    ASTNode::ArrayLiteral(arr) => {
                        assert_eq!(arr.len(), 3);
                        assert!(matches!(arr[0], DataValue::Bool(true)));
                        assert!(matches!(arr[1], DataValue::Bool(false)));
                        assert!(matches!(arr[2], DataValue::Bool(true)));
                    }
                    _ => panic!("Expected array of arguments"),
                }
            }
            _ => panic!("Expected operator token"),
        }
    }

    #[test]
    fn test_lexer_custom_operator() {
        let arena = Bump::new();
        let json_str = r#"{"custom_op": [1, 2, 3]}"#;

        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::CustomOperator { name, args } => {
                assert_eq!(*name, "custom_op");
                match &**args {
                    ASTNode::ArrayLiteral(arr) => {
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

    #[test]
    fn test_lexer_complex() {
        let arena = Bump::new();

        // A more complex JSONLogic expression
        let json_str = r#"
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

        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::Operator { op_type, .. } => {
                assert_eq!(*op_type, OperatorType::If);
            }
            _ => panic!("Expected if operator token"),
        }
    }

    #[test]
    fn test_lexer_nested() {
        let arena = Bump::new();

        // Nested operators
        let json_str = r#"
        {
          "and": [
            {"==": [{"var": "a"}, 1]},
            {"==": [{"var": "b"}, 2]}
          ]
        }
        "#;

        let token = parser(json_str, &arena).unwrap();
        match token {
            ASTNode::Operator { op_type, .. } => {
                assert_eq!(*op_type, OperatorType::And);
            }
            _ => panic!("Expected and operator token"),
        }
    }

    #[test]
    fn test_token_methods() {
        let arena = Bump::new();

        // Test is_operator
        let op_token = Box::new(ASTNode::Operator {
            op_type: OperatorType::Add,
            args: Box::new(ASTNode::Literal(helpers::null())),
        });
        assert!(op_token.is_operator());

        // Test is_literal
        let lit_token = Box::new(ASTNode::Literal(helpers::boolean(true)));
        assert!(lit_token.is_literal());

        // Test is_variable
        let var_token = Box::new(ASTNode::Variable {
            path: arena.alloc(DataValue::String("path")),
            default: None,
            scope_jump: None,
        });
        assert!(var_token.is_variable());

        // Test is_custom_operator
        let custom_token = Box::new(ASTNode::CustomOperator {
            name: arena.alloc_str("custom"),
            args: Box::new(ASTNode::Literal(helpers::null())),
        });
        assert!(custom_token.is_custom_operator());
    }

    #[test]
    fn test_is_static_token() {
        let arena = Bump::new();

        // Test literal - should be static
        let literal = ASTNode::Literal(helpers::int(42));
        assert!(literal.is_static());

        // Test array of literals - should be static
        let array = ASTNode::ArrayLiteral(vec![
            DataValue::Number(Number::Integer(1)),
            DataValue::Number(Number::Integer(2)),
        ]);
        assert!(array.is_static());

        // Test variable - should not be static
        let variable = ASTNode::Variable {
            path: arena.alloc(DataValue::String("foo")),
            default: None,
            scope_jump: None,
        };
        assert!(!variable.is_static());

        // Test dynamic variable - should not be static
        let dynamic_var = ASTNode::DynamicVariable {
            path_expr: Box::new(ASTNode::Literal(helpers::string(&arena, "path"))),
            default: None,
            scope_jump: None,
        };
        assert!(!dynamic_var.is_static());

        // Test operator with static arguments - should be static
        let add_op = ASTNode::Operator {
            op_type: OperatorType::Add,
            args: Box::new(ASTNode::ArrayLiteral(vec![
                DataValue::Number(Number::Integer(1)),
                DataValue::Number(Number::Integer(2)),
            ])),
        };
        assert!(add_op.is_static());

        // Test var operator - should not be static
        let var_op = ASTNode::Operator {
            op_type: OperatorType::Var,
            args: Box::new(ASTNode::Literal(helpers::string(&arena, "foo"))),
        };
        assert!(!var_op.is_static());

        // Test operator with mixed arguments - should be static
        let mixed_op = ASTNode::Operator {
            op_type: OperatorType::Add,
            args: Box::new(ASTNode::ArrayLiteral(vec![
                DataValue::Number(Number::Integer(1)),
                DataValue::String("bar"),
            ])),
        };
        assert!(mixed_op.is_static());

        // Test nested operators - should maintain static status correctly
        let nested_static = ASTNode::Operator {
            op_type: OperatorType::And,
            args: Box::new(ASTNode::Array(vec![
                Box::new(ASTNode::Operator {
                    op_type: OperatorType::Add,
                    args: Box::new(ASTNode::ArrayLiteral(vec![
                        DataValue::Number(Number::Integer(1)),
                        DataValue::Number(Number::Integer(2)),
                    ])),
                }),
                Box::new(ASTNode::Literal(helpers::boolean(true))),
            ])),
        };
        assert!(nested_static.is_static());

        // Test nested operators with variable - should not be static
        let nested_non_static = ASTNode::Operator {
            op_type: OperatorType::And,
            args: Box::new(ASTNode::Array(vec![
                Box::new(ASTNode::Operator {
                    op_type: OperatorType::Add,
                    args: Box::new(ASTNode::ArrayLiteral(vec![
                        DataValue::Number(Number::Integer(1)),
                        DataValue::Number(Number::Integer(2)),
                    ])),
                }),
                Box::new(ASTNode::Variable {
                    path: arena.alloc(DataValue::String("flag")),
                    default: None,
                    scope_jump: None,
                }),
            ])),
        };
        assert!(!nested_non_static.is_static());

        // Test custom operator - should not be static
        let custom_op = ASTNode::CustomOperator {
            name: arena.alloc_str("my_op"),
            args: Box::new(ASTNode::Literal(helpers::int(42))),
        };
        assert!(!custom_op.is_static());
    }
}
