//! Arithmetic operators for JSONLogic
//!
//! This module provides functions for evaluating arithmetic operations like
//! addition, subtraction, multiplication, and division on arrays of values.

use bumpalo::Bump;
use datavalue_rs::{helpers, DataValue, Number, Result};

use crate::DataValueExt;

fn convert_number<'a>(value: f64) -> DataValue<'a> {
    if value.fract() == 0.0 {
        helpers::int(value as i64)
    } else {
        helpers::float(value)
    }
}

/// Evaluates an addition operation on an array of values
///
/// # Arguments
///
/// * `values` - The array of values to add
///
/// # Returns
///
/// A DataValue containing the result of the addition
pub fn evaluate_add<'a>(values: &[DataValue<'a>], arena: &'a Bump) -> Result<&'a DataValue<'a>> {
    // Handle empty array case
    println!("values: {:?}", values);
    if values.is_empty() {
        return Ok(arena.alloc(helpers::int(0)));
    }

    let mut sum = DataValue::Number(Number::Integer(0));

    for val in values {
        sum = (sum + val.coerce_to_number())?;
    }
    Ok(arena.alloc(sum))
}

/// Evaluates a subtraction operation on an array of values
///
/// # Arguments
///
/// * `values` - The array of values to subtract
///
/// # Returns
///
/// A DataValue containing the result of the subtraction
pub fn evaluate_subtract<'a>(
    values: &[DataValue<'a>],
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    // Handle empty array case
    if values.is_empty() {
        return Ok(arena.alloc(helpers::int(0)));
    }

    // Get the first value as the starting point
    let first = match values[0].coerce_to_number() {
        DataValue::Number(Number::Integer(i)) => i as f64,
        DataValue::Number(Number::Float(f)) => f,
        _ => {
            return Err(datavalue_rs::Error::Custom(
                "Invalid starting value".to_string(),
            ))
        }
    };

    if values.len() == 1 {
        return Ok(arena.alloc(convert_number(-first)));
    }

    let sum = values
        .iter()
        .skip(1)
        .fold(first, |acc, val| match val.coerce_to_number() {
            DataValue::Number(Number::Integer(i)) => acc - i as f64,
            DataValue::Number(Number::Float(f)) => acc - f,
            _ => acc,
        });
    Ok(arena.alloc(convert_number(sum)))
}

/// Evaluates a multiplication operation on an array of values
///
/// # Arguments
///
/// * `values` - The array of values to multiply
///
/// # Returns
///
/// A DataValue containing the result of the multiplication
pub fn evaluate_multiply<'a>(
    values: &[DataValue<'a>],
    arena: &'a Bump,
) -> Result<&'a DataValue<'a>> {
    // Handle empty array case
    if values.is_empty() {
        return Ok(arena.alloc(helpers::int(1)));
    }

    let product = values
        .iter()
        .fold(1.0, |acc, val| match val.coerce_to_number() {
            DataValue::Number(Number::Integer(i)) => acc * i as f64,
            DataValue::Number(Number::Float(f)) => acc * f,
            _ => acc,
        });
    Ok(arena.alloc(convert_number(product)))
}

/// Evaluates a division operation on an array of values
///
/// # Arguments
///
/// * `values` - The array of values to divide
///
/// # Returns
///
/// A DataValue containing the result of the division
pub fn evaluate_divide<'a>(values: &[DataValue<'a>], arena: &'a Bump) -> Result<&'a DataValue<'a>> {
    // Handle empty array case
    if values.is_empty() {
        return Ok(arena.alloc(helpers::int(0)));
    }

    let first = match values[0].coerce_to_number() {
        DataValue::Number(Number::Integer(i)) => i as f64,
        DataValue::Number(Number::Float(f)) => f,
        _ => {
            return Err(datavalue_rs::Error::Custom(
                "Invalid starting value".to_string(),
            ))
        }
    };

    if values.len() == 1 {
        return Ok(arena.alloc(convert_number(1.0 / first)));
    }

    let product = values
        .iter()
        .skip(1)
        .fold(first, |acc, val| match val.coerce_to_number() {
            DataValue::Number(Number::Integer(i)) => acc / i as f64,
            DataValue::Number(Number::Float(f)) => acc / f,
            _ => acc,
        });
    Ok(arena.alloc(convert_number(product)))
}

/// Evaluates a modulo operation on an array of values
///
/// # Arguments
///
/// * `values` - The array of values to modulo
///
/// # Returns
///
/// A DataValue containing the result of the modulo
pub fn evaluate_modulo<'a>(values: &[DataValue<'a>], arena: &'a Bump) -> Result<&'a DataValue<'a>> {
    // Handle empty array case
    if values.is_empty() {
        return Ok(arena.alloc(helpers::int(0)));
    }

    let first = match values[0] {
        DataValue::Number(Number::Integer(i)) => i as f64,
        DataValue::Number(Number::Float(f)) => f,
        _ => {
            return Err(datavalue_rs::Error::Custom(
                "Invalid starting value".to_string(),
            ))
        }
    };

    let product = values
        .iter()
        .skip(1)
        .fold(first, |acc, val| match val.coerce_to_number() {
            DataValue::Number(Number::Integer(i)) => acc % i as f64,
            DataValue::Number(Number::Float(f)) => acc % f,
            _ => acc,
        });
    Ok(arena.alloc(convert_number(product)))
}

/// Evaluates a minimum operation on an array of values
///
/// # Arguments
///
/// * `values` - The array of values to find the minimum of
///
/// # Returns
///
/// A DataValue containing the result of the minimum
pub fn evaluate_min<'a>(values: &[DataValue<'a>], arena: &'a Bump) -> Result<&'a DataValue<'a>> {
    // Handle empty array case
    if values.is_empty() {
        return Ok(arena.alloc(helpers::int(0)));
    }

    let min = values
        .iter()
        .fold(f64::MAX, |acc, val| match val.coerce_to_number() {
            DataValue::Number(Number::Integer(i)) => acc.min(i as f64),
            DataValue::Number(Number::Float(f)) => acc.min(f),
            _ => acc,
        });
    Ok(arena.alloc(convert_number(min)))
}

/// Evaluates a maximum operation on an array of values
///
/// # Arguments
///
/// * `values` - The array of values to find the maximum of
///
/// # Returns
///
/// A DataValue containing the result of the maximum
pub fn evaluate_max<'a>(values: &[DataValue<'a>], arena: &'a Bump) -> Result<&'a DataValue<'a>> {
    // Handle empty array case
    if values.is_empty() {
        return Ok(arena.alloc(helpers::int(0)));
    }

    let max = values
        .iter()
        .fold(f64::MIN, |acc, val| match val.coerce_to_number() {
            DataValue::Number(Number::Integer(i)) => acc.max(i as f64),
            DataValue::Number(Number::Float(f)) => acc.max(f),
            _ => acc,
        });
    Ok(arena.alloc(convert_number(max)))
}

/// Evaluates a absolute value operation on an array of values
///
/// # Arguments
///
/// * `values` - The array of values to evaluate the absolute value of
///
/// # Returns
///
/// A DataValue containing the result of the absolute value
pub fn evaluate_abs<'a>(values: &[DataValue<'a>], arena: &'a Bump) -> Result<&'a DataValue<'a>> {
    // Handle empty array case
    if values.is_empty() {
        return Ok(arena.alloc(helpers::int(0)));
    }

    let mut result = Vec::with_capacity(values.len());

    for val in values {
        let num = val.coerce_to_number();
        match num {
            DataValue::Number(Number::Integer(i)) => result.push(helpers::int(i.abs())),
            DataValue::Number(Number::Float(f)) => result.push(helpers::float(f.abs())),
            _ => {}
        }
    }

    Ok(arena.alloc(DataValue::Array(arena.alloc_slice_fill_iter(result))))
}

/// Evaluates a ceiling operation on an array of values
///
/// # Arguments
///
/// * `values` - The array of values to evaluate the ceiling of
///
/// # Returns
///
/// A DataValue containing the result of the ceiling
pub fn evaluate_ceil<'a>(values: &[DataValue<'a>], arena: &'a Bump) -> Result<&'a DataValue<'a>> {
    // Handle empty array case
    if values.is_empty() {
        return Ok(arena.alloc(helpers::int(0)));
    }

    let mut result = Vec::with_capacity(values.len());

    for val in values {
        let num = val.coerce_to_number();
        match num {
            DataValue::Number(Number::Integer(i)) => result.push(helpers::int(i)),
            DataValue::Number(Number::Float(f)) => result.push(helpers::float(f.ceil())),
            _ => {}
        }
    }

    Ok(arena.alloc(DataValue::Array(arena.alloc_slice_fill_iter(result))))
}

/// Evaluates a floor operation on an array of values
///
/// # Arguments
///
/// * `values` - The array of values to evaluate the floor of
///
/// # Returns
///
/// A DataValue containing the result of the floor
pub fn evaluate_floor<'a>(values: &[DataValue<'a>], arena: &'a Bump) -> Result<&'a DataValue<'a>> {
    // Handle empty array case
    if values.is_empty() {
        return Ok(arena.alloc(helpers::int(0)));
    }

    let mut result = Vec::with_capacity(values.len());

    for val in values {
        let num = val.coerce_to_number();
        match num {
            DataValue::Number(Number::Integer(i)) => result.push(helpers::int(i)),
            DataValue::Number(Number::Float(f)) => result.push(helpers::float(f.floor())),
            _ => {}
        }
    }

    Ok(arena.alloc(DataValue::Array(arena.alloc_slice_fill_iter(result))))
}
