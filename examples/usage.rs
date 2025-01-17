use datalogic_rs::*;
use serde_json::json;

fn main() {
	let logic = json!({ "max": {"var": "data"} });
	let data = json!({ "data": [1, 2, 3] });
	println!("Logic: {:?}", logic);

	let rule = Rule::from_value(&logic).unwrap();
	println!("Rule: {:?}", rule);
	let result = JsonLogic::apply(&rule, &data).unwrap();
	println!("Result: {}", result);
}