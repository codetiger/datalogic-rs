# datalogic-rs

A high-performance JSONLogic implementation in Rust.

## Features

- **Fast Parsing**: Efficiently parses JSONLogic expressions using arena allocation
- **Token Optimization**: Optimizes parsed tokens by flattening nested operations and performing constant folding
- **Memory Efficient**: Uses `DataValue` from the `datavalue-rs` crate for efficient memory usage

## Usage

### Basic Usage

```rust
use datalogic_rs::{parser, optimize, Bump};

fn main() {
    // Create an arena for allocation
    let arena = Bump::new();
    
    // Parse a JSONLogic expression
    let json = r#"{"+":[1, {"+":[2, 3]}, 4]}"#;
    let token = parser(json, &arena).unwrap();
    
    // Optimize the token tree
    let optimized = optimize(&*token, &arena);
    
    // Now optimized contains a flattened token that evaluates to 10
}
```

### Optimizations Performed

The optimizer provides several performance enhancements:

1. **Arithmetic Flattening**: Nested arithmetic operations of the same type are flattened
   - `{"+":[1, {"+":[2, 3]}]}` → `{"+": [1, 2, 3]}`

2. **Logical Operation Flattening**: Nested AND/OR operations are flattened
   - `{"and":[a, {"and":[b, c]}]}` → `{"and": [a, b, c]}`

3. **Constant Folding**: Operations on constant values are evaluated at optimization time
   - `{"+":[1, 2, 3]}` → `6`
   - `{"*":[2, 3, 4]}` → `24`

4. **Short-Circuit Optimization**: Logical operations are short-circuited when possible
   - `{"and":[false, <complex_expr>]}` → `false`
   - `{"or":[true, <complex_expr>]}` → `true`

## License

This project is licensed under the MIT License - see the LICENSE file for details.

