//! Stack-based Virtual Machine implementation for JSONLogic
//!
//! This module provides a simplified stack-based VM for executing
//! JSONLogic rules compiled to bytecode.

#![forbid(unsafe_code)]

use crate::compiler::types::{CallTag, OpCode, OpTag, Program};
use crate::value::datavalue_rs::{DataValue, Number};
use crate::DataValueExt;
use bumpalo::Bump;

/// Variable lookup trait for VM contexts
pub trait VarLookup {
    /// Look up a variable in the context
    fn lookup(&self, name: &str) -> Option<&DataValue>;
}

/// Data context for VM execution
pub struct DataContext<'a> {
    data: &'a DataValue<'a>,
}

impl<'a> DataContext<'a> {
    /// Create a new data context
    pub fn new(data: &'a DataValue<'a>) -> Self {
        Self { data }
    }

    /// Get the data
    pub fn data(&self) -> &DataValue<'a> {
        self.data
    }
}

impl<'a> VarLookup for DataContext<'a> {
    fn lookup(&self, _name: &str) -> Option<&DataValue> {
        Some(self.data)
    }
}

/// Execute a program with the given context and arena
pub fn run<'a>(
    program: &'a Program<'a>,
    context: &'a dyn VarLookup,
    arena: &'a Bump,
) -> DataValue<'a> {
    let mut stack: Vec<DataValue<'a>> = Vec::with_capacity(16);
    let mut ip = 0;

    while ip < program.instructions.len() {
        let instr = &program.instructions[ip];
        match instr.opcode() {
            OpCode::LoadConst => {
                let idx = instr.imm() as usize;
                if idx < program.const_pool.len() {
                    let value = &program.const_pool[idx];
                    stack.push(value.clone());
                } else {
                    println!(
                        "  LoadConst: Index {} out of bounds (max {})",
                        idx,
                        program.const_pool.len() - 1
                    );
                }
            }
            OpCode::LoadVar => {
                let idx = instr.imm() as usize;
                if idx < program.const_pool.len() {
                    let value = &program.const_pool[idx];
                    let data = context.lookup("").unwrap();

                    // Try to get the default value from constant pool
                    let default_idx = idx + 1; // Default value should be the next item in the pool
                    let default_value = if default_idx < program.const_pool.len() {
                        Some(&program.const_pool[default_idx])
                    } else {
                        None
                    };

                    let scope_jump_idx = default_idx + 1;
                    let _scope_jump = if scope_jump_idx < program.const_pool.len() {
                        Some(&program.const_pool[scope_jump_idx])
                    } else {
                        None
                    };

                    // Get the value from the data context
                    let mut found = false;
                    let mut result = DataValue::Null;

                    match *value {
                        DataValue::Null | DataValue::String("") | DataValue::Array([]) => {
                            result = data.clone();
                            found = true;
                        }
                        DataValue::String(s) => {
                            if let Some(val) = data.get(s) {
                                result = val.clone();
                                found = true;
                            }
                        }
                        DataValue::Number(Number::Integer(i)) => {
                            if let Some(val) = data.get_index(i as usize) {
                                result = val.clone();
                                found = true;
                            }
                        }
                        DataValue::Array(arr) => {
                            result = data.clone();
                            for item in arr {
                                match *item {
                                    DataValue::String(s) => {
                                        if let Some(val) = result.get(s) {
                                            result = val.clone();
                                            found = true;
                                        } else {
                                            result = DataValue::Null;
                                            found = false;
                                        }
                                    }
                                    DataValue::Number(Number::Integer(i)) => {
                                        if let Some(val) = result.get_index(i as usize) {
                                            result = val.clone();
                                            found = true;
                                        } else {
                                            result = DataValue::Null;
                                            found = false;
                                        }
                                    }
                                    _ => {
                                        result = DataValue::Null;
                                        found = false;
                                    }
                                }
                            }
                        }
                        _ => {
                            println!("  LoadVar: Unsupported type for variable lookup");
                        }
                    }

                    // If value not found, use default if available
                    if !found && default_value.is_some() {
                        result = default_value.unwrap().clone();
                    }

                    stack.push(result);
                } else {
                    println!(
                        "  LoadVar: Index {} out of bounds (max {})",
                        idx,
                        program.const_pool.len() - 1
                    );
                }
            }
            OpCode::LoadDynamicVar => {
                // For dynamic variable lookup, the top of the stack should contain:
                // 1. Path (string or array) - dynamic path computed at runtime
                // 2. Default value (any)
                // 3. Scope jump (integer)

                // Pop scope jump, default, and path from stack
                let _scope_jump = stack
                    .pop()
                    .unwrap_or(DataValue::Number(Number::Integer(-1)));
                let default_value = stack.pop().unwrap_or(DataValue::Null);
                let path = stack.pop().unwrap_or(DataValue::Null);

                // Get the data context
                let data = context.lookup("").unwrap();

                // Get the value from the data context
                let mut found = false;
                let mut result = DataValue::Null;

                // Process the path to get the value
                match path {
                    DataValue::Null | DataValue::String("") | DataValue::Array([]) => {
                        result = data.clone();
                        found = true;
                    }
                    DataValue::String(s) => {
                        if let Some(val) = data.get(s) {
                            result = val.clone();
                            found = true;
                        } else {
                            // Try to handle dot notation paths like "pie.filling"
                            if s.contains('.') {
                                let parts: Vec<&str> = s.split('.').collect();
                                let mut current = data;
                                found = true;

                                for part in parts {
                                    if let Some(val) = current.get(part) {
                                        current = val;
                                    } else {
                                        found = false;
                                        println!(
                                            "  LoadDynamicVar: {} not found in path {}",
                                            part, s
                                        );
                                        break;
                                    }
                                }

                                if found {
                                    result = current.clone();
                                }
                            } else {
                                println!("  LoadDynamicVar: {} not found", s);
                            }
                        }
                    }
                    DataValue::Number(Number::Integer(i)) => {
                        if let Some(val) = data.get_index(i as usize) {
                            result = val.clone();
                            found = true;
                        } else {
                            println!("  LoadDynamicVar: {} not found", i);
                        }
                    }
                    DataValue::Array(arr) => {
                        result = data.clone();
                        for item in arr {
                            match *item {
                                DataValue::String(s) => {
                                    if let Some(val) = result.get(s) {
                                        result = val.clone();
                                        found = true;
                                    } else {
                                        result = DataValue::Null;
                                        found = false;
                                    }
                                }
                                DataValue::Number(Number::Integer(i)) => {
                                    if let Some(val) = result.get_index(i as usize) {
                                        result = val.clone();
                                        found = true;
                                    } else {
                                        result = DataValue::Null;
                                        found = false;
                                    }
                                }
                                _ => {
                                    result = DataValue::Null;
                                    found = false;
                                }
                            }
                        }
                    }
                    _ => {
                        println!("  LoadDynamicVar: Unsupported type for variable lookup");
                    }
                }

                // If value not found, use default
                if !found {
                    result = default_value;
                }

                // Push result to stack
                stack.push(result);
            }
            OpCode::Variadic => {
                // Extract operation tag (high 8 bits) and argument count (low 16 bits)
                let op_tag = (instr.imm() >> 16) as u8;
                let arg_count = (instr.imm() & 0xFFFF) as usize;

                // Ensure we have enough arguments on the stack
                if stack.len() < arg_count {
                    // Not enough arguments, push default value based on operation
                    match OpTag::from(op_tag as u32) {
                        OpTag::Add => stack.push(DataValue::Number(Number::Integer(0))),
                        OpTag::Mul => stack.push(DataValue::Number(Number::Integer(1))),
                        OpTag::And => stack.push(DataValue::Bool(true)),
                        OpTag::Or => stack.push(DataValue::Bool(false)),
                        _ => stack.push(DataValue::Null),
                    }
                    continue;
                }

                // Extract exactly the number of arguments specified
                let mut args = Vec::with_capacity(arg_count);
                for _ in 0..arg_count {
                    if let Some(val) = stack.pop() {
                        if let DataValue::Array(arr) = val {
                            args.extend(arr.iter().cloned());
                        } else {
                            args.push(val);
                        }
                    } else {
                        args.push(DataValue::Null); // Fallback in case of stack issues
                    }
                }

                // println!("  Variadic: args={:?}", args);

                // Process based on operation tag
                match OpTag::from(op_tag as u32) {
                    OpTag::Add => {
                        let mut result = DataValue::Number(Number::Float(0.0));
                        for arg in args {
                            // Safely handle addition and null/invalid values
                            match result.clone() + arg.coerce_to_number() {
                                Ok(val) => result = val,
                                Err(_) => {
                                    println!("  Variadic: Error during addition, skipping operand");
                                    // Just continue with current value
                                }
                            }
                        }
                        stack.push(result.convert_to_number());
                    }
                    OpTag::Sub => {
                        if args.is_empty() {
                            stack.push(DataValue::Number(Number::Integer(0)));
                        } else if args.len() == 1 {
                            // Unary negation: -x
                            let result = (DataValue::Number(Number::Integer(0))
                                - args[0].coerce_to_number())
                            .unwrap();
                            stack.push(result);
                        } else {
                            // Now that args are properly ordered, args[0] is the first argument (left side of the operator)
                            // and args[1] is the second argument (right side of the operator)
                            let mut result = args[0].coerce_to_number();
                            for arg in &args[1..] {
                                match result.clone() - arg.coerce_to_number() {
                                    Ok(val) => result = val,
                                    Err(_) => {
                                        println!("  Variadic: Error during subtraction");
                                    }
                                }
                            }
                            stack.push(result);
                        }
                    }
                    OpTag::Mul => {
                        let mut result = DataValue::Number(Number::Integer(1));
                        for arg in args {
                            match result.clone() * arg.coerce_to_number() {
                                Ok(val) => result = val,
                                Err(_) => {
                                    println!("  Variadic: Error during multiplication");
                                    // Continue with current value
                                }
                            }
                        }
                        stack.push(result);
                    }
                    OpTag::Div => {
                        if args.is_empty() {
                            stack.push(DataValue::Number(Number::Integer(0)));
                        } else if args.len() == 1 {
                            // Reciprocal: 1/x
                            let result = (DataValue::Number(Number::Float(1.0))
                                / args[0].coerce_to_number())
                            .unwrap();
                            stack.push(result);
                        } else {
                            // Now that args are properly ordered, args[0] is the first argument (left side of the operator)
                            // and args[1] is the second argument (right side of the operator)
                            let mut result = args[0].coerce_to_number();
                            for arg in &args[1..] {
                                // Avoid division by zero or invalid operands
                                let arg_num = arg.coerce_to_number();
                                match arg_num {
                                    DataValue::Number(Number::Integer(i)) if i == 0 => {
                                        println!("  Variadic: Division by zero");
                                        result = DataValue::Number(Number::Integer(0));
                                        break;
                                    }
                                    DataValue::Number(Number::Float(f)) if f == 0.0 => {
                                        println!("  Variadic: Division by zero");
                                        result = DataValue::Number(Number::Integer(0));
                                        break;
                                    }
                                    _ => {}
                                }
                                match result.clone() / arg.coerce_to_number() {
                                    Ok(val) => result = val,
                                    Err(_) => {
                                        println!("  Variadic: Error during division");
                                    }
                                }
                            }
                            stack.push(result);
                        }
                    }
                    OpTag::Min => {
                        if args.is_empty() {
                            stack.push(DataValue::Number(Number::Integer(0)));
                        } else {
                            let mut result = args[0].clone();
                            for arg in &args[1..] {
                                if result > *arg {
                                    result = arg.clone();
                                }
                            }
                            stack.push(result);
                        }
                    }
                    OpTag::Max => {
                        if args.is_empty() {
                            stack.push(DataValue::Number(Number::Integer(0)));
                        } else {
                            let mut result = args[0].clone();
                            for arg in &args[1..] {
                                if result < *arg {
                                    result = arg.clone();
                                }
                            }
                            stack.push(result);
                        }
                    }
                    OpTag::Mod => {
                        stack.push(args[0].modulo(&args[1]));
                    }
                    OpTag::And => {
                        // For JSONLogic, "and" returns the first falsy value,
                        // or the first value if all are truthy (corrected behavior)
                        if args.is_empty() {
                            stack.push(DataValue::Bool(true)); // Default value for empty args
                        } else {
                            let mut result_value = args[0].clone();

                            for arg in &args[1..] {
                                if !result_value.is_truthy() {
                                    break; // Found falsy value, stop and return it
                                }
                                // Only update result if previous was truthy
                                result_value = arg.clone();
                            }

                            // Push the actual value (not just a boolean)
                            stack.push(result_value);
                        }
                    }
                    OpTag::Or => {
                        // For JSONLogic, "or" returns the first truthy value,
                        // or the last value if all are falsy
                        let mut result_value = DataValue::Bool(false);

                        for arg in args {
                            result_value = arg.clone();
                            if arg.is_truthy() {
                                break;
                            }
                        }

                        // Push the actual value (not just a boolean)
                        stack.push(result_value);
                    }
                    OpTag::LessThan => {
                        // Variadic less than - all adjacent pairs must be in ascending order
                        if args.len() < 2 {
                            stack.push(DataValue::Bool(false));
                        } else {
                            let mut result = true;
                            // Compare each adjacent pair: args[i] < args[i+1]
                            for i in 0..args.len() - 1 {
                                let comparison = args[i].compare(&args[i + 1]);
                                if comparison != std::cmp::Ordering::Less {
                                    result = false;
                                    break;
                                }
                            }
                            stack.push(DataValue::Bool(result));
                        }
                    }
                    OpTag::LessThanOrEqual => {
                        // Variadic less than or equal - all adjacent pairs must be in non-descending order
                        if args.len() < 2 {
                            stack.push(DataValue::Bool(false));
                        } else {
                            let mut result = true;
                            // Compare each adjacent pair: args[i] <= args[i+1]
                            for i in 0..args.len() - 1 {
                                let comparison = args[i].compare(&args[i + 1]);
                                if comparison != std::cmp::Ordering::Less
                                    && comparison != std::cmp::Ordering::Equal
                                {
                                    result = false;
                                    break;
                                }
                            }
                            stack.push(DataValue::Bool(result));
                        }
                    }
                    OpTag::GreaterThan => {
                        // Variadic greater than - all adjacent pairs must be in descending order
                        if args.len() < 2 {
                            stack.push(DataValue::Bool(false));
                        } else {
                            let mut result = true;
                            // Compare each adjacent pair: args[i] > args[i+1]
                            for i in 0..args.len() - 1 {
                                let comparison = args[i].compare(&args[i + 1]);
                                if comparison != std::cmp::Ordering::Greater {
                                    result = false;
                                    break;
                                }
                            }
                            stack.push(DataValue::Bool(result));
                        }
                    }
                    OpTag::GreaterThanOrEqual => {
                        // Variadic greater than or equal - all adjacent pairs must be in non-ascending order
                        if args.len() < 2 {
                            stack.push(DataValue::Bool(false));
                        } else {
                            let mut result = true;
                            // Compare each adjacent pair: args[i] >= args[i+1]
                            for i in 0..args.len() - 1 {
                                let comparison = args[i].compare(&args[i + 1]);
                                if comparison != std::cmp::Ordering::Greater
                                    && comparison != std::cmp::Ordering::Equal
                                {
                                    result = false;
                                    break;
                                }
                            }
                            stack.push(DataValue::Bool(result));
                        }
                    }
                    OpTag::Equal => {
                        // Variadic equality - all values must be equal to each other
                        if args.len() < 2 {
                            stack.push(DataValue::Bool(false));
                        } else {
                            let mut result = true;
                            // Compare each value against the first value
                            for i in 1..args.len() {
                                if !args[0].loose_equals(&args[i]) {
                                    result = false;
                                    break;
                                }
                            }
                            stack.push(DataValue::Bool(result));
                        }
                    }
                    OpTag::NotEqual => {
                        // Variadic inequality - all values must be different from each other
                        if args.len() < 2 {
                            stack.push(DataValue::Bool(false));
                        } else {
                            let mut result = true;
                            // Compare each pair to ensure they're all different
                            for i in 0..args.len() {
                                for j in i + 1..args.len() {
                                    if args[i].loose_equals(&args[j]) {
                                        result = false;
                                        break;
                                    }
                                }
                                if !result {
                                    break;
                                }
                            }
                            stack.push(DataValue::Bool(result));
                        }
                    }
                    OpTag::StrictEqual => {
                        // Variadic strict equality - all values must be strictly equal to each other
                        if args.len() < 2 {
                            stack.push(DataValue::Bool(false));
                        } else {
                            let mut result = true;
                            // Compare each value against the first value
                            for i in 1..args.len() {
                                if !args[0].strict_equals(&args[i]) {
                                    result = false;
                                    break;
                                }
                            }
                            stack.push(DataValue::Bool(result));
                        }
                    }
                    OpTag::StrictNotEqual => {
                        // Variadic strict inequality - all values must be strictly different from each other
                        if args.len() < 2 {
                            stack.push(DataValue::Bool(false));
                        } else {
                            let mut result = true;
                            // Compare each pair to ensure they're all different
                            for i in 0..args.len() {
                                for j in i + 1..args.len() {
                                    if args[i].strict_equals(&args[j]) {
                                        result = false;
                                        break;
                                    }
                                }
                                if !result {
                                    break;
                                }
                            }
                            stack.push(DataValue::Bool(result));
                        }
                    }
                    OpTag::In => {
                        // In operator: checks if value is in array or string
                        // Two possible patterns:
                        // 1. args[0] = needle, args[1] = haystack
                        // 2. args[0] = needle, args[1...n] = individual array elements to search in

                        if args.len() < 2 {
                            stack.push(DataValue::Bool(false));
                        } else {
                            // Get the needle (the first argument)
                            let needle = &args[0];

                            // Check if the second argument is an array or string
                            let result = match &args[1] {
                                // If the second arg is an array, use it directly as the haystack
                                DataValue::Array(arr) => {
                                    arr.iter().any(|item| needle.loose_equals(item))
                                }

                                // If the second arg is a string, check if needle is in the string
                                DataValue::String(s) => {
                                    match needle {
                                        // Single character search
                                        DataValue::String(c) if c.len() == 1 => s.contains(c),
                                        // Substring search
                                        DataValue::String(substr) => s.contains(substr),
                                        // Convert numbers to string for comparison
                                        DataValue::Number(_) => {
                                            if let Some(c) = needle.to_string().chars().next() {
                                                s.contains(c)
                                            } else {
                                                false
                                            }
                                        }
                                        // Other types aren't supported for string searching
                                        _ => false,
                                    }
                                }
                                // Other container types aren't supported
                                _ => false,
                            };

                            stack.push(DataValue::Bool(result));
                        }
                    }
                    OpTag::Not => {
                        // Handle the case when args is empty
                        if args.is_empty() {
                            stack.push(DataValue::Bool(true));
                        } else {
                            // The args are in reverse order, so we need to grab the first argument
                            // which is the last item in args
                            stack.push(DataValue::Bool(!args[args.len() - 1].is_truthy()));
                        }
                    }
                    OpTag::DNot => {
                        // Handle the case when args is empty
                        if args.is_empty() {
                            stack.push(DataValue::Bool(false));
                        } else {
                            // The args are in reverse order, so we need to grab the first argument
                            // which is the last item in args
                            stack.push(DataValue::Bool(args[args.len() - 1].is_truthy()));
                        }
                    }
                }
            }
            OpCode::Jump => {
                let target = instr.imm() as usize;
                // Set ip to target - 1 because it will be incremented at the end of the loop
                ip = target - 1;
            }
            OpCode::JumpIfFalse => {
                let target = instr.imm() as usize;
                // Pop condition from stack
                let condition = stack.pop().unwrap_or(DataValue::Null);

                if !condition.is_truthy() {
                    // Set ip to target - 1 because it will be incremented at the end of the loop
                    ip = target - 1;
                }
            }
            OpCode::Return => {
                // Return is handled below by breaking out of the loop
            }
            OpCode::Call => {
                // Get the tag from the immediate value
                let tag = instr.imm() as u8;

                if tag == 0 {
                    // Create an array from all items currently on the stack
                    let values: Vec<DataValue<'a>> = stack.drain(..).collect();
                    let array_slice = arena.alloc_slice_clone(&values);
                    stack.push(DataValue::Array(array_slice));
                } else {
                    // Process based on the call tag
                    match CallTag::from_u8(tag).unwrap() {
                        // Merge - combines multiple arrays into a single array
                        // Usage: {"merge": [array1, array2, ...]}
                        CallTag::Merge => {
                            // Need at least one argument
                            if stack.is_empty() {
                                stack.push(DataValue::Array(&[]));
                                continue;
                            }

                            // Collect all items to be merged
                            let mut result: Vec<DataValue<'a>> = Vec::new();

                            // Process each argument
                            while !stack.is_empty() {
                                match stack.pop().unwrap() {
                                    // If argument is an array, extend result with its items
                                    DataValue::Array(arr) => {
                                        result.extend(arr.iter().cloned());
                                    }
                                    // For non-array arguments, just add them directly
                                    item => result.push(item),
                                }
                            }

                            // Create the merged array
                            let array_slice = arena.alloc_slice_clone(&result);
                            stack.push(DataValue::Array(array_slice));
                        }
                        // Cat - concatenates strings
                        // Usage: {"cat": [str1, str2, ...]}
                        CallTag::Cat => {
                            // Need at least one argument
                            if stack.is_empty() {
                                stack.push(DataValue::String(""));
                                continue;
                            }

                            // Collect all items to be concatenated
                            let mut items = Vec::new();

                            // Pop all arguments
                            while !stack.is_empty() {
                                items.push(stack.pop().unwrap());
                            }

                            // Build the concatenated string
                            let mut result = String::new();
                            for item in items {
                                match item {
                                    DataValue::String(s) => result.push_str(s),
                                    DataValue::Number(Number::Integer(i)) => {
                                        result.push_str(&i.to_string())
                                    }
                                    DataValue::Number(Number::Float(f)) => {
                                        result.push_str(&f.to_string())
                                    }
                                    DataValue::Bool(b) => result.push_str(&b.to_string()),
                                    DataValue::Null => result.push_str("null"),
                                    _ => result.push_str(&format!("{:?}", item)),
                                }
                            }

                            // Create string in arena
                            let string_in_arena = arena.alloc_str(&result);
                            stack.push(DataValue::String(string_in_arena));
                        }
                        // Substr - extract a substring
                        // Usage: {"substr": [string, start, (optional) length]}
                        CallTag::Substring => {
                            // Need at least 2 arguments (string and start)
                            if stack.len() < 2 {
                                stack.push(DataValue::String(""));
                                continue;
                            }

                            // IMPORTANT: In the compiled code, constants are loaded in reverse order
                            // so we need to adjust our handling here:
                            // The stack will have (from top to bottom): string, start, [length]

                            // Pop values in the order they appear on the stack
                            let string = stack.pop().unwrap_or(DataValue::String(""));
                            let start =
                                stack.pop().unwrap_or(DataValue::Number(Number::Integer(0)));
                            let length_opt = if stack.len() >= 1 { stack.pop() } else { None };

                            // Convert to string if not already a string
                            let source = match string {
                                DataValue::String(s) => s.to_string(),
                                _ => string.to_string(),
                            };

                            let chars: Vec<char> = source.chars().collect();
                            let str_len = chars.len();

                            // Process start index
                            let start_idx = match start {
                                DataValue::Number(Number::Integer(i)) => {
                                    if i < 0 {
                                        // Negative indices count from the end
                                        str_len.saturating_sub(i.unsigned_abs() as usize)
                                    } else {
                                        i as usize
                                    }
                                }
                                _ => 0, // Non-number start, default to 0
                            };

                            // Process length
                            let length = match length_opt {
                                Some(DataValue::Number(Number::Integer(i))) => {
                                    if i < 0 {
                                        // Negative length means count from end of string
                                        if start_idx < str_len {
                                            let end_pos =
                                                str_len.saturating_sub(i.unsigned_abs() as usize);
                                            if end_pos > start_idx {
                                                end_pos - start_idx
                                            } else {
                                                0
                                            }
                                        } else {
                                            0
                                        }
                                    } else {
                                        i as usize
                                    }
                                }
                                _ => str_len.saturating_sub(start_idx), // No length or non-number length
                            };

                            // Extract the substring
                            let result = if start_idx < str_len {
                                let end_idx = (start_idx + length).min(str_len);
                                chars[start_idx..end_idx].iter().collect::<String>()
                            } else {
                                String::new()
                            };

                            // Create string in arena
                            let string_in_arena = arena.alloc_str(&result);
                            stack.push(DataValue::String(string_in_arena));
                        }
                        // Missing - Check which variables are missing from the data context
                        // Usage: {"missing": ["var1", "var2", "path.to.var", ...]}
                        CallTag::Missing => {
                            // The stack will contain either direct paths or an array of paths
                            // from a complex expression like merge
                            
                            // Collect all paths to check
                            let mut paths = Vec::new();
                            
                            // Handle both cases:
                            // 1. Multiple individual path arguments on the stack
                            // 2. A single array of paths on the stack (from a complex expression)
                            if stack.len() == 1 {
                                match stack.pop().unwrap() {
                                    DataValue::Array(arr) => {
                                        // Got an array from a complex expression
                                        paths.extend(arr.iter().cloned());
                                    },
                                    other => {
                                        // Single path
                                        paths.push(other);
                                    }
                                }
                            } else {
                                // Multiple individual paths on the stack
                                while !stack.is_empty() {
                                    paths.push(stack.pop().unwrap());
                                }
                            }
                            
                            // Get the data context
                            let data = context.lookup("").unwrap();
                            
                            // Check which paths are missing and collect them
                            let mut missing_paths = Vec::new();
                            
                            for path in paths {
                                let mut is_missing = true;
                                
                                match path {
                                    DataValue::String(s) => {
                                        // Check if the path exists in the data
                                        if s.is_empty() {
                                            // Empty string refers to the whole data context
                                            is_missing = false;
                                        } else if s.contains('.') {
                                            // Handle dot notation like "path.to.var"
                                            let parts: Vec<&str> = s.split('.').collect();
                                            let mut current = data;
                                            let mut found = true;
                                            
                                            for part in parts.iter() {
                                                if let Some(val) = current.get(*part) {
                                                    current = val;
                                                } else {
                                                    found = false;
                                                    break;
                                                }
                                            }
                                            
                                            // If we made it through all parts without finding a missing part
                                            is_missing = !found;
                                        } else if data.get(s).is_some() {
                                            // Direct property access
                                            is_missing = false;
                                        }
                                    }
                                    DataValue::Number(Number::Integer(i)) => {
                                        // Check array index
                                        if data.get_index(i as usize).is_some() {
                                            is_missing = false;
                                        }
                                    }
                                    DataValue::Array(arr) => {
                                        // For array paths like ['a', 'b', 'c'], try to follow the path
                                        let mut current = data;
                                        let mut found = true;
                                        
                                        for item in arr {
                                            match item {
                                                DataValue::String(s) => {
                                                    if let Some(val) = current.get(s) {
                                                        current = val;
                                                    } else {
                                                        found = false;
                                                        break;
                                                    }
                                                }
                                                DataValue::Number(Number::Integer(i)) => {
                                                    if let Some(val) = current.get_index(*i as usize) {
                                                        current = val;
                                                    } else {
                                                        found = false;
                                                        break;
                                                    }
                                                }
                                                _ => {
                                                    found = false;
                                                    break;
                                                }
                                            }
                                        }
                                        
                                        is_missing = !found;
                                    }
                                    _ => {
                                        // Unsupported path type, consider it missing
                                        is_missing = true;
                                    }
                                }
                                
                                // If the path is missing, add it to the result
                                if is_missing {
                                    missing_paths.push(path);
                                }
                            }
                            
                            // Create the result array of missing paths
                            let array_slice = arena.alloc_slice_clone(&missing_paths);
                            stack.push(DataValue::Array(array_slice));
                        }
                        // MissingSome - Check if at least N variables are present, return missing otherwise
                        // Usage: {"missing_some": [min_required, ["var1", "var2", ...]]}
                        CallTag::MissingSome => {
                            // Handle both cases:
                            // 1. Standard usage with min_required and paths as separate arguments
                            // 2. Complex expression that produces a single array [min_required, paths]
                            
                            let mut min_required = 0;
                            let mut paths = Vec::new();
                            
                            if stack.len() == 1 {
                                // Case 2: A single array from a complex expression
                                if let DataValue::Array(arr) = stack.pop().unwrap() {
                                    if arr.len() >= 2 {
                                        // Extract min_required from the first element
                                        min_required = match arr[0] {
                                            DataValue::Number(Number::Integer(i)) => i as usize,
                                            _ => 0,
                                        };
                                        
                                        // Extract paths from the second element
                                        if let DataValue::Array(path_arr) = &arr[1] {
                                            paths.extend(path_arr.iter().cloned());
                                        } else {
                                            // If second element isn't an array, treat it as a single path
                                            paths.push(arr[1].clone());
                                        }
                                    }
                                }
                            } else if stack.len() >= 2 {
                                // Case 1: Standard usage with separate arguments on the stack
                                // IMPORTANT: The stack has the arguments in REVERSE order:
                                // - Top of stack: number (min_required)
                                // - Next: array of paths
                                
                                // First pop the min_required count from the stack (top item)
                                min_required = match stack.pop().unwrap() {
                                    DataValue::Number(Number::Integer(i)) => i as usize,
                                    _ => 0,
                                };
                                
                                // Then pop the paths array
                                let paths_arg = stack.pop().unwrap();
                                match paths_arg {
                                    DataValue::Array(arr) => {
                                        paths.extend(arr.iter().cloned());
                                    },
                                    other => {
                                        paths.push(other);
                                    }
                                }
                            } else {
                                // Not enough arguments, return empty array
                                stack.push(DataValue::Array(&[]));
                                continue;
                            }
                            
                            // Get the data context
                            let data = context.lookup("").unwrap();
                            
                            // Track found and missing paths
                            let mut found_count = 0;
                            let mut missing_paths = Vec::new();
                            
                            // Check each path
                            for path in paths {
                                let mut is_missing = true;
                                
                                match &path {
                                    DataValue::String(s) => {
                                        // Check if the path exists in the data
                                        if s.is_empty() {
                                            // Empty string refers to the whole data context
                                            is_missing = false;
                                        } else if s.contains('.') {
                                            // Handle dot notation like "path.to.var"
                                            let parts: Vec<&str> = s.split('.').collect();
                                            let mut current = data;
                                            let mut found = true;
                                            
                                            for part in parts.iter() {
                                                if let Some(val) = current.get(*part) {
                                                    current = val;
                                                } else {
                                                    found = false;
                                                    break;
                                                }
                                            }
                                            
                                            is_missing = !found;
                                        } else if data.get(s).is_some() {
                                            // Direct property access
                                            is_missing = false;
                                        }
                                    }
                                    DataValue::Number(Number::Integer(i)) => {
                                        // Check array index
                                        if data.get_index(*i as usize).is_some() {
                                            is_missing = false;
                                        }
                                    }
                                    DataValue::Array(arr) => {
                                        // For array paths like ['a', 'b', 'c'], try to follow the path
                                        let mut current = data;
                                        let mut found = true;
                                        
                                        for item in arr.iter() {
                                            match item {
                                                DataValue::String(s) => {
                                                    if let Some(val) = current.get(s) {
                                                        current = val;
                                                    } else {
                                                        found = false;
                                                        break;
                                                    }
                                                }
                                                DataValue::Number(Number::Integer(i)) => {
                                                    if let Some(val) = current.get_index(*i as usize) {
                                                        current = val;
                                                    } else {
                                                        found = false;
                                                        break;
                                                    }
                                                }
                                                _ => {
                                                    found = false;
                                                    break;
                                                }
                                            }
                                        }
                                        
                                        is_missing = !found;
                                    }
                                    _ => {
                                        // Unsupported path type, consider it missing
                                        is_missing = true;
                                    }
                                }
                                
                                // Count found paths and collect missing ones
                                if !is_missing {
                                    found_count += 1;
                                } else {
                                    missing_paths.push(path);
                                }
                            }
                            
                            // If we found enough paths, return an empty array
                            // Otherwise return the array of missing paths
                            if found_count >= min_required {
                                stack.push(DataValue::Array(&[]));
                            } else {
                                let array_slice = arena.alloc_slice_clone(&missing_paths);
                                stack.push(DataValue::Array(array_slice));
                            }
                        }
                        // For unhandled tags, don't modify the stack
                        _ => {
                            println!("  Call: Unhandled call tag: {}", tag);
                        }
                    }
                }
            }
            // Default fallback for other opcodes
            _ => {
                // Push null for other unimplemented opcodes
                println!("  Unhandled opcode: {:?}", instr.opcode());
                stack.push(DataValue::Null);
            }
        }

        ip += 1;
    }

    if stack.is_empty() {
        return DataValue::Null;
    } else if stack.len() == 1 {
        return stack.pop().unwrap();
    } else {
        // This is a more reasonable default behavior - return the top item on the stack
        // If an array was intended, it would have been explicitly created
        return stack.pop().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::compile;
    use crate::core::parser;
    use datavalue_rs::DataValue;

    #[test]
    fn test_simple_addition() {
        let arena = Bump::new();

        // Parse {"+":[3,4]}
        let rule_str = r#"{"+":[3,4]}"#;
        let ast = parser(rule_str, &arena).unwrap();

        // Compile to bytecode
        let program = compile(ast, &arena).unwrap();

        // Execute
        let data = arena.alloc(DataValue::Null);
        let context = DataContext::new(data);
        let result = run(&program, &context, &arena);

        // Verify result
        match result {
            DataValue::Number(Number::Integer(val)) => assert_eq!(val, 7),
            _ => panic!("Expected integer result"),
        }
    }
    
    #[test]
    fn test_missing_operator() {
        let arena = Bump::new();

        // Parse {"missing":["a", "b", "c"]}
        let rule_str = r#"{"missing":["a", "b", "c"]}"#;
        let ast = parser(rule_str, &arena).unwrap();

        // Compile to bytecode
        let program = compile(ast, &arena).unwrap();

        // Create data with "b" present
        let data_json = r#"{"b": 123}"#;
        let data = DataValue::from_str(&arena, data_json).unwrap();
        let context = DataContext::new(&data);
        
        // Execute
        let result = run(&program, &context, &arena);

        // Verify result - should return ["a", "c"]
        match result {
            DataValue::Array(missing) => {
                assert_eq!(missing.len(), 2);
                // Check that "a" and "c" are in the missing array
                let has_a = missing.iter().any(|item| {
                    if let DataValue::String(s) = item {
                        *s == "a"
                    } else {
                        false
                    }
                });
                let has_c = missing.iter().any(|item| {
                    if let DataValue::String(s) = item {
                        *s == "c"
                    } else {
                        false
                    }
                });
                assert!(has_a, "Missing array should contain 'a'");
                assert!(has_c, "Missing array should contain 'c'");
            },
            _ => panic!("Expected array result"),
        }
    }
    
    #[test]
    fn test_missing_some_operator() {
        let arena = Bump::new();

        // Parse {"missing_some":[1, ["a", "b", "c"]]}
        let rule_str = r#"{"missing_some":[1, ["a", "b", "c"]]}"#;
        let ast = parser(rule_str, &arena).unwrap();

        // Compile to bytecode
        let program = compile(ast, &arena).unwrap();

        // Case 1: Test with "b" present - should return empty array since min_required=1
        let data_json = r#"{"b": 123}"#;
        let data = DataValue::from_str(&arena, data_json).unwrap();
        let context = DataContext::new(&data);
        
        // Execute
        let result = run(&program, &context, &arena);

        // Verify result - should return []
        match result {
            DataValue::Array(missing) => {
                assert_eq!(missing.len(), 0, "Should return empty array when 1 required var is present");
            },
            _ => panic!("Expected array result"),
        }
        
        // Case 2: Test with no variables present - should return all missing vars
        let arena = Bump::new();
        let rule_str = r#"{"missing_some":[2, ["a", "b", "c"]]}"#;
        let ast = parser(rule_str, &arena).unwrap();
        let program = compile(ast, &arena).unwrap();
        
        let data_json = r#"{}"#;
        let data = DataValue::from_str(&arena, data_json).unwrap();
        let context = DataContext::new(&data);
        
        // Execute
        let result = run(&program, &context, &arena);

        // Verify result - when none of the required variables are present, should return all missing variables
        match result {
            DataValue::Array(missing) => {
                // We need 2 variables present but we have 0, so we should get all 3 back
                assert_eq!(missing.len(), 3, "Should return all missing vars when none are present");
                
                // Check that "a", "b", and "c" are in the missing array
                let has_a = missing.iter().any(|item| {
                    if let DataValue::String(s) = item {
                        *s == "a"
                    } else {
                        false
                    }
                });
                let has_b = missing.iter().any(|item| {
                    if let DataValue::String(s) = item {
                        *s == "b"
                    } else {
                        false
                    }
                });
                let has_c = missing.iter().any(|item| {
                    if let DataValue::String(s) = item {
                        *s == "c"
                    } else {
                        false
                    }
                });
                
                assert!(has_a, "Missing array should contain 'a'");
                assert!(has_b, "Missing array should contain 'b'");
                assert!(has_c, "Missing array should contain 'c'");
            },
            _ => panic!("Expected array result"),
        }
    }
}
