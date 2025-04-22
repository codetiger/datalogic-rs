use bumpalo::Bump;
use datalogic_rs::{engine, parser};
use datavalue_rs::DataValue;

fn main() {
    // Create memory arena
    let arena = Bump::new();

    let rule = r#"{"+":[{"+": [1, 2]}, {"+": [3, 4]}]}"#;

    // Parse the rule
    let token = parser::parser(rule, &arena).expect("Failed to parse rule");
    println!("Token: {:?}", token);

    // Create data context (empty for this example since rule doesn't reference variables)
    let data = DataValue::Null;

    // Evaluate the rule
    let result = engine::evaluate(token, &data, &arena).expect("Failed to evaluate rule");

    // Print the result
    println!("Rule: {}", rule);
    println!("Result: {}", result);
}
