//! Tests for the stack-based VM implementation

use crate::{
    compiler::compile,
    core::{parser, ASTNode},
    vm_stack::{run, DataContext, VarLookup},
    DataBump,
};
use datavalue_rs::{helpers, DataValue, Number};

#[test]
fn test_simple_arithmetic() {
    let arena = DataBump::new();
    
    // Parse JSON rule: {"+":[3,4]}
    let json = r#"{"+":[3,4]}"#;
    let ast = parser(json, &arena).expect("Failed to parse");
    
    // Compile the AST
    let program = compile(ast, &arena).expect("Failed to compile");
    
    // Create a data context (empty for this example)
    let data = arena.alloc(DataValue::Null);
    let context = DataContext::new(data);
    
    // Run the program
    let result = run(&program, &context, &arena);
    
    // Assert the result
    match result {
        DataValue::Number(Number::Integer(7)) => {}, // success
        _ => panic!("Expected 7, got {:?}", result),
    }
}

#[test]
fn test_variable_access() {
    let arena = DataBump::new();
    
    // Parse JSON rule: {"var": "x"}
    let json = r#"{"var": "x"}"#;
    let ast = parser(json, &arena).expect("Failed to parse");
    
    // Compile the AST
    let program = compile(ast, &arena).expect("Failed to compile");
    
    // Create data with x=42
    let mut obj = Vec::new();
    obj.push(("x", helpers::int(42)));
    let data = arena.alloc(DataValue::Object(&obj));
    let context = DataContext::new(data);
    
    // Run the program
    let result = run(&program, &context, &arena);
    
    // Assert the result
    match result {
        DataValue::Number(Number::Integer(42)) => {}, // success
        _ => panic!("Expected 42, got {:?}", result),
    }
}

#[test]
fn test_if_statement() {
    let arena = DataBump::new();
    
    // Parse JSON rule: {"if":[{"var":"x"},{"*":[{"var":"x"},2]},0]}
    let json = r#"{"if":[{"var":"x"},{"*":[{"var":"x"},2]},0]}"#;
    let ast = parser(json, &arena).expect("Failed to parse");
    
    // Compile the AST
    let program = compile(ast, &arena).expect("Failed to compile");
    
    // Test with x=5
    let mut obj = Vec::new();
    obj.push(("x", helpers::int(5)));
    let data = arena.alloc(DataValue::Object(&obj));
    let context = DataContext::new(data);
    
    // Run the program
    let result = run(&program, &context, &arena);
    
    // Assert the result
    match result {
        DataValue::Number(Number::Integer(10)) => {}, // success (x=5, so x*2=10)
        _ => panic!("Expected 10, got {:?}", result),
    }
    
    // Test with x=0 (falsy)
    let mut obj2 = Vec::new();
    obj2.push(("x", helpers::int(0)));
    let data2 = arena.alloc(DataValue::Object(&obj2));
    let context2 = DataContext::new(data2);
    
    // Run the program again
    let result2 = run(&program, &context2, &arena);
    
    // Assert the result
    match result2 {
        DataValue::Number(Number::Integer(0)) => {}, // success (x=0 is falsy, so return 0)
        _ => panic!("Expected 0, got {:?}", result2),
    }
}

#[test]
fn test_comparison_operators() {
    let arena = DataBump::new();
    
    // Test equality: {"==":[{"var":"a"},{"var":"b"}]}
    let json = r#"{"==":[{"var":"a"},{"var":"b"}]}"#;
    let ast = parser(json, &arena).expect("Failed to parse");
    let program = compile(ast, &arena).expect("Failed to compile");
    
    // Test with a=1, b=1
    let mut obj = Vec::new();
    obj.push(("a", helpers::int(1)));
    obj.push(("b", helpers::int(1)));
    let data = arena.alloc(DataValue::Object(&obj));
    let context = DataContext::new(data);
    
    let result = run(&program, &context, &arena);
    match result {
        DataValue::Bool(true) => {}, // success
        _ => panic!("Expected true, got {:?}", result),
    }
    
    // Test with a=1, b=2
    let mut obj2 = Vec::new();
    obj2.push(("a", helpers::int(1)));
    obj2.push(("b", helpers::int(2)));
    let data2 = arena.alloc(DataValue::Object(&obj2));
    let context2 = DataContext::new(data2);
    
    let result2 = run(&program, &context2, &arena);
    match result2 {
        DataValue::Bool(false) => {}, // success
        _ => panic!("Expected false, got {:?}", result2),
    }
}

#[test]
fn test_logical_operators() {
    let arena = DataBump::new();
    
    // Test AND: {"and":[{"var":"a"},{"var":"b"}]}
    let json = r#"{"and":[{"var":"a"},{"var":"b"}]}"#;
    let ast = parser(json, &arena).expect("Failed to parse");
    let program = compile(ast, &arena).expect("Failed to compile");
    
    // Test with a=true, b=true
    let mut obj = Vec::new();
    obj.push(("a", helpers::boolean(true)));
    obj.push(("b", helpers::boolean(true)));
    let data = arena.alloc(DataValue::Object(&obj));
    let context = DataContext::new(data);
    
    let result = run(&program, &context, &arena);
    match result {
        DataValue::Bool(true) => {}, // success
        _ => panic!("Expected true, got {:?}", result),
    }
    
    // Test with a=true, b=false
    let mut obj2 = Vec::new();
    obj2.push(("a", helpers::boolean(true)));
    obj2.push(("b", helpers::boolean(false)));
    let data2 = arena.alloc(DataValue::Object(&obj2));
    let context2 = DataContext::new(data2);
    
    let result2 = run(&program, &context2, &arena);
    match result2 {
        DataValue::Bool(false) => {}, // success
        _ => panic!("Expected false, got {:?}", result2),
    }
}

#[test]
fn test_nested_expressions() {
    let arena = DataBump::new();
    
    // Test nested expressions: {"if": [{"<": [{"var": "temp"}, 0]}, "freezing", {"if": [{"<": [{"var": "temp"}, 100]}, "liquid", "gas"]}]}
    let json = r#"{"if": [{"<": [{"var": "temp"}, 0]}, "freezing", {"if": [{"<": [{"var": "temp"}, 100]}, "liquid", "gas"]}]}"#;
    let ast = parser(json, &arena).expect("Failed to parse");
    let program = compile(ast, &arena).expect("Failed to compile");
    
    // Test with temp=-10
    let mut obj = Vec::new();
    obj.push(("temp", helpers::int(-10)));
    let data = arena.alloc(DataValue::Object(&obj));
    let context = DataContext::new(data);
    
    let result = run(&program, &context, &arena);
    match result {
        DataValue::String(s) if s == "freezing" => {}, // success
        _ => panic!("Expected 'freezing', got {:?}", result),
    }
    
    // Test with temp=25
    let mut obj2 = Vec::new();
    obj2.push(("temp", helpers::int(25)));
    let data2 = arena.alloc(DataValue::Object(&obj2));
    let context2 = DataContext::new(data2);
    
    let result2 = run(&program, &context2, &arena);
    match result2 {
        DataValue::String(s) if s == "liquid" => {}, // success
        _ => panic!("Expected 'liquid', got {:?}", result2),
    }
    
    // Test with temp=150
    let mut obj3 = Vec::new();
    obj3.push(("temp", helpers::int(150)));
    let data3 = arena.alloc(DataValue::Object(&obj3));
    let context3 = DataContext::new(data3);
    
    let result3 = run(&program, &context3, &arena);
    match result3 {
        DataValue::String(s) if s == "gas" => {}, // success
        _ => panic!("Expected 'gas', got {:?}", result3),
    }
} 