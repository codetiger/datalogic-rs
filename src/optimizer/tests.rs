//! Tests for the optimizer module
//!
//! This module contains tests for the token optimization functionality.

#[cfg(test)]
mod tests {
    use crate::optimizer::optimize;
    use crate::parser::{parser, OperatorType, Token};
    use bumpalo::Bump;
    use datavalue_rs::{DataValue, Number};

    /// Helper function to parse JSON into a token
    fn parse_token<'a>(json: &str, arena: &'a Bump) -> Box<Token<'a>> {
        Box::new((*parser(json, arena).unwrap()).clone())
    }

    #[test]
    fn test_optimize_add_flatten() {
        let arena = Bump::new();

        // Test nested addition operations
        let token = parse_token(r#"{"+":[1, {"+":[2, 3]}]}"#, &arena);
        let optimized = optimize(&token, &arena);

        // Check the result (should be a literal 6 or flattened operation)
        match &*optimized {
            Token::Literal(DataValue::Number(Number::Integer(i))) => {
                // If constant folding worked, we get a single integer
                assert_eq!(*i, 6);
            }
            Token::Operator { op_type, args } => {
                // If only flattening worked, we get a flattened operator
                assert_eq!(*op_type, OperatorType::Add);

                // The args should be an array with 3 elements
                match &**args {
                    Token::ArrayLiteral(tokens) => {
                        assert_eq!(tokens.len(), 3);
                    }
                    _ => panic!("Expected array literal"),
                }
            }
            _ => panic!("Expected either literal or operator token"),
        }
    }

    #[test]
    fn test_optimize_nested_arithmetic() {
        let arena = Bump::new();

        // Test deeply nested arithmetic operations
        let token = parse_token(r#"{"+":[1, {"+":[2, {"+":[3, 4]}]}]}"#, &arena);
        let optimized = optimize(&token, &arena);

        // Check if all nested additions are flattened
        match &*optimized {
            Token::Literal(DataValue::Number(Number::Integer(i))) => {
                // If constant folding worked, we get a single integer
                assert_eq!(*i, 10);
            }
            Token::Operator { op_type, args } => {
                assert_eq!(*op_type, OperatorType::Add);

                // The args should be an array with all literals flattened
                match &**args {
                    Token::ArrayLiteral(tokens) => {
                        // Should have 4 tokens (1, 2, 3, 4)
                        assert_eq!(tokens.len(), 4);
                    }
                    _ => panic!("Expected array literal"),
                }
            }
            _ => panic!("Expected either literal or operator token"),
        }
    }

    #[test]
    fn test_optimize_logical_and() {
        let arena = Bump::new();

        // Test AND with nested AND (no longer short-circuits in optimizer)
        let token = parse_token(r#"{"and":[true, {"and":[true, false]}]}"#, &arena);

        // Debug the input token
        println!("Input token: {:?}", token);

        let optimized = optimize(&token, &arena);

        // Debug the optimized token
        println!("Optimized token: {:?}", optimized);

        // Should be a flattened AND operation
        match &*optimized {
            Token::Operator { op_type, args } => {
                assert_eq!(*op_type, OperatorType::And);

                // The args should be flattened
                match &**args {
                    Token::ArrayLiteral(tokens) => {
                        // Should have 3 tokens (true, true, false)
                        println!("Number of tokens: {}", tokens.len());
                        for (i, t) in tokens.iter().enumerate() {
                            println!("Token {}: {:?}", i, t);
                        }
                        assert_eq!(tokens.len(), 3);
                    }
                    _ => panic!("Expected array literal"),
                }
            }
            _ => panic!("Expected operator token"),
        }
    }

    #[test]
    fn test_optimize_logical_or() {
        let arena = Bump::new();

        // Test OR with early true value (no longer short-circuits in optimizer)
        let token = parse_token(r#"{"or":[false, true, {"complex": "expression"}]}"#, &arena);
        let optimized = optimize(&token, &arena);

        // Should be an OR operator with flattened arguments
        match &*optimized {
            Token::Operator { op_type, args } => {
                assert_eq!(*op_type, OperatorType::Or);

                // The args should be an array with the original elements
                match &**args {
                    Token::ArrayLiteral(tokens) => {
                        // Should have 3 tokens
                        assert_eq!(tokens.len(), 3);
                    }
                    _ => panic!("Expected array literal"),
                }
            }
            _ => panic!("Expected operator token"),
        }
    }

    #[test]
    fn test_optimize_logical_or_flattening() {
        let arena = Bump::new();

        // Test OR with nested OR
        let token = parse_token(r#"{"or":[false, {"or":[false, {"var": "x"}]}]}"#, &arena);
        let optimized = optimize(&token, &arena);

        // Should be flattened OR operation
        match &*optimized {
            Token::Operator { op_type, args } => {
                assert_eq!(*op_type, OperatorType::Or);

                // The args should be flattened
                match &**args {
                    Token::ArrayLiteral(tokens) => {
                        // Should have 3 tokens (false, false, var)
                        assert_eq!(tokens.len(), 3);

                        // Verify the last token is a variable reference
                        match &*tokens[2] {
                            Token::Variable { path, .. } => match path {
                                DataValue::String(s) => assert_eq!(*s, "x"),
                                _ => panic!("Expected string path"),
                            },
                            _ => panic!("Expected variable token"),
                        }
                    }
                    _ => panic!("Expected array literal"),
                }
            }
            _ => panic!("Expected operator token"),
        }
    }

    #[test]
    fn test_optimize_no_change() {
        let arena = Bump::new();

        // Test a token that doesn't need optimization
        let token = parse_token(r#"{"var": "user.name"}"#, &arena);
        let optimized = optimize(&token, &arena);

        // Should stay a variable token
        match &*optimized {
            Token::Variable { path, .. } => {
                // Handle both string and array paths (for dotted notation)
                match path {
                    DataValue::String(s) => assert_eq!(*s, "user.name"),
                    DataValue::Array(arr) => {
                        // Check if this is a path array for dot notation
                        if arr.len() == 2 {
                            if let DataValue::String(first) = &arr[0] {
                                if let DataValue::String(second) = &arr[1] {
                                    assert_eq!(*first, "user");
                                    assert_eq!(*second, "name");
                                    return;
                                }
                            }
                        }
                        panic!("Unexpected array path format");
                    }
                    _ => panic!("Expected string or array path"),
                }
            }
            _ => panic!("Expected variable token"),
        }
    }
}
