use datalogic_rs::*;
use serde_json::json;

fn main() {
    let logic = json!({
        "==": [
            {"var": "user.age"},
            21
        ]
    });
    
    let data = json!({
        "user": {
            "age": 21,
            "name": "John"
        }
    });

    let rule = Rule::from_value(&logic).unwrap();
    let result = JsonLogic::apply(&rule, &data).unwrap();
    println!("Is user 21? {}", result);  // Prints: Is user 21? true
}