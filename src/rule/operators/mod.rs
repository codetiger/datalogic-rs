use serde_json::Value;
use super::ArgType;
use super::Rule;
use super::Error;

pub mod arithmetic;
pub mod array;
pub mod comparison;
pub mod logic;
pub mod missing;
pub mod preserve;
pub mod string;
pub mod var;
pub mod control;
pub mod val;
pub mod tryop;
pub mod custom;

pub use arithmetic::*;
pub use array::*;
pub use comparison::*;
pub use logic::*;
pub use missing::*;
pub use preserve::*;
pub use string::*;
pub use var::*;
pub use control::*;
pub use val::*;
pub use tryop::*;
pub use custom::*;


trait ValueCoercion {
    fn is_null_value(&self) -> bool;
    fn coerce_to_bool(&self) -> bool;
    fn coerce_to_number(&self) -> Result<f64, Error>;
    fn coerce_to_string(&self) -> String;
    fn coerce_append(result: &mut String, value: &Value);
}

impl ValueCoercion for Value {
    fn coerce_to_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Number(n) => {
                let num = n.as_f64().unwrap_or(0.0);
                num != 0.0
            },
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(_) => true,
            Value::Null => false,
        }
    }

    fn coerce_to_number(&self) -> Result<f64, Error> {
        match self {
            Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0)),
            Value::String(s) if s.is_empty() => Ok(0.0),
            Value::String(s) => s.parse::<f64>().map_err(|_| Error::Custom("NaN".to_string())),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Value::Null => Ok(0.0),
            Value::Array(_) | Value::Object(_) => Err(Error::Custom("NaN".to_string())),
        }
    }

    fn coerce_to_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => {
                let mut result = String::with_capacity(arr.len() * 8);
                for item in arr.iter() {
                    Self::coerce_append(&mut result, item);
                }
                result
            },
            Value::Object(_) => "[object Object]".to_string(),
        }
    }

    fn coerce_append(result: &mut String, value: &Value) {
        match value {
            Value::String(s) => result.push_str(s),
            Value::Number(n) => result.push_str(&n.to_string()),
            Value::Bool(b) => result.push_str(if *b { "true" } else { "false" }),
            Value::Null => result.push_str("null"),
            Value::Array(arr) => {
                for item in arr.iter() {
                    Self::coerce_append(result, item);
                }
            },
            Value::Object(_) => result.push_str("[object Object]"),
        }
    }

    fn is_null_value(&self) -> bool {
        match self {
            Value::Bool(_) => false,
            Value::Number(_) => false,
            Value::String(s) => s.is_empty(),
            Value::Array(a) => a.is_empty(),
            Value::Object(o) => o.is_empty(),
            Value::Null => true,
        }
    }
}

trait ValueConvert {
    fn to_value(&self) -> Value;
}

impl ValueConvert for f64 {
    fn to_value(&self) -> Value {
        const ZERO_FRACT: f64 = 0.0;
        if self.fract() == ZERO_FRACT {
            Value::Number(serde_json::Number::from(*self as i64))
        } else {
            Value::Number(serde_json::Number::from_f64(*self).unwrap())
        }
    }
}

#[inline(always)]
fn is_current_var(var_name: &Rule) -> bool {
    match var_name {
        Rule::Value(Value::String(name)) => name == "current",
        Rule::Value(Value::Array(arr)) if !arr.is_empty() => {
            if let Some(Value::String(first)) = arr.first() {
                first == "current"
            } else {
                false
            }
        }
        _ => false
    }
}

#[inline]
pub fn is_flat_arithmetic_predicate(rule: &Rule) -> bool {
    if let Rule::Arithmetic(op_type, ArgType::Multiple(args)) = rule {
        if args.len() == 2 {
            // Check if operation is arithmetic
            if !matches!(op_type, 
                ArithmeticType::Add | 
                ArithmeticType::Subtract | 
                ArithmeticType::Multiply | 
                ArithmeticType::Divide | 
                ArithmeticType::Modulo
            ) {
                return false;
            }

            // Check if we have one current/current.* and one accumulator using array_windows
            let (has_current, has_accumulator) = args.iter().fold(
                (false, false),
                |mut acc, arg| {
                    if let Rule::Var(var_name, _) = arg {
                        match &**var_name {
                            Rule::Value(Value::String(name)) if name == "accumulator" => {
                                acc.1 = true;
                            }
                            _ if is_current_var(var_name) => {
                                acc.0 = true;
                            }
                            _ => {}
                        }
                    }
                    acc
                }
            );

            return has_current && has_accumulator;
        }
    }
    false
}