use datalogic_rs::*;
use serde_json::Value;
use std::fs;
use std::time::Instant;

fn main() {
    // Load test cases from JSON file
    let response =
        fs::read_to_string("tests/suites/compatible.json").expect("Failed to read test cases file");

    let json_data: Vec<Value> =
        serde_json::from_str(&response).expect("Failed to parse test cases");

    // Create logic engine for parsing and compilation
    let compile_engine = DataLogic::new();

    // Extract rules and data
    let mut test_cases = Vec::new();
    for entry in json_data {
        // Skip string entries (comments)
        if entry.is_string() {
            continue;
        }

        if let Value::Object(test_case) = entry {
            // Get rule and data
            if let Some(logic) = test_case.get("rule") {
                let data = test_case.get("data").unwrap_or(&Value::Null);
                let data_str = data.to_string();
                let data_value = compile_engine.parse_data(data_str.as_str()).unwrap();

                let rule_json_str = logic.to_string();
                let compiled = compile_engine.compile(&rule_json_str).unwrap();

                // Just store the string representations to parse later
                test_cases.push((compiled, data_value));
            }
        }
    }

    let iterations = 1e5 as u32; // Reduced iterations to avoid OOM
    println!(
        "Running {} iterations for {} test cases",
        iterations,
        test_cases.len()
    );

    let start = Instant::now();
    let mut evaluate_engine = DataLogic::new();

    // Run benchmark with precompiled logic
    for (compiled, data_value) in &test_cases {
        // Apply the precompiled rule repeatedly
        for _ in 0..iterations {
            let _ = evaluate_engine.apply_logic(&compiled, &data_value);
        }

        // Reset after each test case to free memory
        evaluate_engine.reset();
    }

    let duration_compiled = start.elapsed();
    println!("\nPrecompiled evaluation:");
    println!("Total time: {:?}", duration_compiled);
    println!(
        "Average iteration time: {:?}",
        duration_compiled / (iterations * test_cases.len() as u32)
    );
    println!(
        "Iterations per second: {:.2}",
        (iterations * test_cases.len() as u32) as f64 / duration_compiled.as_secs_f64()
    );
}
