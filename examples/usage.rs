use bumpalo::Bump;
use datalogic_rs::{
    compiler, core,
    vm_stack::{self, DataContext},
};
use datavalue_rs::DataValue;

fn main() {
    // Create memory arena for all allocations
    let arena = Bump::new();

    // Example 1: Simple Addition Rule
    let rule = r#"{"all":[{"var":"integers"},{">=":[{"var":""},1]}]}"#;
    println!("Rule: {}", rule);

    // Parse and compile the rule
    if let Ok(token) = core::parser(rule, &arena) {
        if let Ok(program) = compiler::compile(token, &arena) {
            println!("Program: {}", program.to_string());
            let data = DataValue::from_str(&arena, r#"{"integers":[1,2,3]}"#).unwrap();
            let context = DataContext::new(&data);
            let result = vm_stack::run(&program, &context, &arena);
            println!("Result: {}", result);
        } else {
            println!("Failed to compile the rule");
        }
    } else {
        println!("Failed to parse the rule");
    }
}
