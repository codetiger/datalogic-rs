use bumpalo::Bump;
use datalogic_rs::core;
use datalogic_rs::engine::InstructionStack;

fn main() {
    println!("=== JSONLogic Instruction Stack Demonstration ===\n");

    // Create memory arena
    let arena = Bump::new();

    // Example 1: Simple Addition Rule
    let addition_rule = r#"{"+":[1, 2, {"var": "x"}]}"#;
    println!("Example 1: {}", addition_rule);

    // Parse the rule and create an instruction stack
    let token = core::parser(addition_rule, &arena).expect("Failed to parse rule");
    let mut stack = InstructionStack::new(token);

    // Print the initial state (uncompiled)
    println!("Initial instruction stack: {:?}", stack.instructions);

    // Compile the instruction stack
    stack
        .compile()
        .expect("Failed to compile instruction stack");

    // Print the compiled instructions
    println!("Compiled instruction stack: {:?}\n", stack.instructions);

    // Example 2: Conditional Rule
    let if_rule = r#"{"if": [{"<": [{"var": "temp"}, 0]}, "freezing", "not freezing"]}"#;
    println!("Example 2: {}", if_rule);

    // Parse and compile
    let if_token = core::parser(if_rule, &arena).expect("Failed to parse if rule");
    let mut if_stack = InstructionStack::new(if_token);

    // Print initial state
    println!("Initial instruction stack: {:?}", if_stack.instructions);

    // Compile the instruction stack
    if_stack.compile().expect("Failed to compile if stack");

    // Print compiled instructions
    println!("Compiled instruction stack: {:?}\n", if_stack.instructions);

    // Example 3: Logic AND Rule
    let and_rule = r#"{"and": [true, {"var": "x"}, 42]}"#;
    println!("Example 3: {}", and_rule);

    // Parse and compile
    let and_token = core::parser(and_rule, &arena).expect("Failed to parse AND rule");
    let mut and_stack = InstructionStack::new(and_token);

    // Print initial and compiled states
    println!("Initial instruction stack: {:?}", and_stack.instructions);
    and_stack.compile().expect("Failed to compile AND stack");
    println!("Compiled instruction stack: {:?}\n", and_stack.instructions);

    // Explain instruction types
    println!("=== Instruction Types ===");
    println!("1. Evaluate - Pushes the result of evaluating a token onto the value stack");
    println!("2. CollectArray - Pops N values from the stack and creates an array");
    println!("3. CollectOperatorArgs - Pops N values from the stack and applies an operator");
    println!("4. EvaluateLazyOperator - Evaluates operators with short-circuit behavior");

    println!(
        "\nNote: The actual evaluation of these instructions is handled by the library internally."
    );
}
