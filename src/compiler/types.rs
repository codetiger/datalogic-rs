//! Common types used in the compiler and VM.
//!
//! This module defines the core structures for the bytecode format,
//! including OpCode and Instr which are used to represent the bytecode
//! instructions.

use datavalue_rs::DataValue;
use std::fmt;
use thiserror::Error;

/// Errors that can occur during compilation.
#[derive(Error, Debug)]
pub enum CompileError {
    #[error("AST optimization error: {0}")]
    OptimizationError(String),

    #[error("Lowering error: {0}")]
    LoweringError(String),

    #[error("Constant pool error: {0}")]
    ConstPoolError(String),

    #[error("Instruction limit exceeded: {0} instructions")]
    InstructionLimitExceeded(usize),

    #[error("Value not found in const pool: {0}")]
    ValueNotInConstPool(String),

    #[error("Locals limit exceeded")]
    LocalsLimitExceeded,
}

/// The bytecode operation codes as defined in the VM specification.
/// Each opcode fits in a single byte (u8).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    // load / store operations
    LoadConst = 0x01,
    LoadLocal = 0x02,
    StoreLocal = 0x03,
    LoadVar = 0x04,
    LoadDynamicVar = 0x05,

    // arithmetic / logic operations
    Variadic = 0x12,
    Call = 0x13,

    // control flow operations
    Jump = 0x20,
    JumpIfFalse = 0x21,
    JumpIfTrue = 0x22,

    // termination
    Return = 0xFF,
}

impl OpCode {
    /// Convert a u8 value to an OpCode
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(OpCode::LoadConst),
            0x02 => Some(OpCode::LoadLocal),
            0x03 => Some(OpCode::StoreLocal),
            0x04 => Some(OpCode::LoadVar),
            0x05 => Some(OpCode::LoadDynamicVar),
            0x12 => Some(OpCode::Variadic),
            0x13 => Some(OpCode::Call),
            0x20 => Some(OpCode::Jump),
            0x21 => Some(OpCode::JumpIfFalse),
            0x22 => Some(OpCode::JumpIfTrue),
            0xFF => Some(OpCode::Return),
            _ => None,
        }
    }

    /// Get a human-readable description of the opcode
    pub fn description(&self) -> &'static str {
        match self {
            OpCode::LoadConst => "Load constant from pool",
            OpCode::LoadLocal => "Load local variable",
            OpCode::StoreLocal => "Store value to local variable",
            OpCode::LoadVar => "Load variable from data context",
            OpCode::LoadDynamicVar => "Load variable with dynamic path",
            OpCode::Variadic => "Apply operation with variable number of arguments",
            OpCode::Call => "Call function",
            OpCode::Jump => "Jump to instruction offset",
            OpCode::JumpIfFalse => "Jump if top of stack is falsy",
            OpCode::JumpIfTrue => "Jump if top of stack is truthy",
            OpCode::Return => "Return top of stack as result",
        }
    }
}

/// A fixed 32-bit instruction encoding.
///
/// The instruction is encoded as follows:
/// - The high 8 bits (bits 24-31) contain the opcode
/// - The low 24 bits (bits 0-23) contain the immediate value
#[derive(Debug, Clone, Copy)]
pub struct Instr(pub u32);

impl Instr {
    /// Create a new instruction from an opcode and immediate value
    pub fn new(opcode: OpCode, imm: u32) -> Self {
        // Ensure immediate value fits in 24 bits
        let imm = imm & 0x00FF_FFFF;
        // Combine opcode and immediate value
        let instruction = ((opcode as u32) << 24) | imm;
        Instr(instruction)
    }

    /// Extract the opcode from the instruction
    pub fn opcode(&self) -> OpCode {
        let op_byte = (self.0 >> 24) as u8;
        OpCode::from_u8(op_byte).unwrap_or_else(|| panic!("Invalid opcode: {}", op_byte))
    }

    /// Extract the immediate value from the instruction
    pub fn imm(&self) -> u32 {
        self.0 & 0x00FF_FFFF
    }

    /// Set the operand/immediate value of the instruction, preserving the opcode
    pub fn set_operand(&mut self, imm: u32) {
        // Keep the opcode bits, clear and set the immediate bits
        let opcode_bits = self.0 & 0xFF00_0000;
        let masked_imm = imm & 0x00FF_FFFF;
        self.0 = opcode_bits | masked_imm;
    }

    /// Format the immediate value based on the opcode type
    pub fn format_immediate(&self) -> String {
        let imm = self.imm();
        match self.opcode() {
            OpCode::LoadConst => format!("{} (const index)", imm),
            OpCode::LoadLocal | OpCode::StoreLocal => format!("{} (local var)", imm),
            OpCode::LoadVar => format!("{} (var path)", imm),
            OpCode::LoadDynamicVar => format!("{} (dynamic path)", imm),
            OpCode::Variadic => {
                // The high 8 bits contain the operation tag, low 16 bits contain argument count
                let op_tag = (imm >> 16) as u8;
                let arg_count = imm & 0xFFFF;

                let op_name = if op_tag <= 0x30 {
                    let tag = OpTag::from(op_tag as u32);
                    format!("{:?}", tag)
                } else {
                    format!("Unknown-{:X}", op_tag)
                };

                format!("{} ({} args)", op_name, arg_count)
            }
            OpCode::Call => {
                if let Some(tag) = CallTag::from_u8(imm as u8) {
                    format!("{:?}", tag)
                } else {
                    format!("{:X} (unknown tag)", imm)
                }
            }
            OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpIfTrue => format!("{} (offset)", imm),
            OpCode::Return => format!("{}", imm),
        }
    }
}

/// Tags for operations used in the Binary and Variadic opcodes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpTag {
    Add = 0x00,
    Sub = 0x01,
    Mul = 0x02,
    Div = 0x03,
    Mod = 0x04,
    Min = 0x05,
    Max = 0x06,

    Equal = 0x10,
    NotEqual = 0x11,
    StrictEqual = 0x12,
    StrictNotEqual = 0x13,
    LessThan = 0x14,
    LessThanOrEqual = 0x15,
    GreaterThan = 0x16,
    GreaterThanOrEqual = 0x17,
    And = 0x20,
    Or = 0x21,
    In = 0x30,

    Not = 0x40,
    DNot = 0x41,
}

impl OpTag {
    pub fn description(&self) -> &'static str {
        match self {
            OpTag::Add => "Add values",
            OpTag::Sub => "Subtract values",
            OpTag::Mul => "Multiply values",
            OpTag::Div => "Divide values",
            OpTag::Mod => "Modulo operation",
            OpTag::Min => "Minimum of values",
            OpTag::Max => "Maximum of values",

            OpTag::Equal => "Equal comparison",
            OpTag::NotEqual => "Not equal comparison",
            OpTag::StrictEqual => "Strict equal comparison",
            OpTag::StrictNotEqual => "Strict not equal comparison",
            OpTag::LessThan => "Less than comparison",
            OpTag::LessThanOrEqual => "Less than or equal comparison",
            OpTag::GreaterThan => "Greater than comparison",
            OpTag::GreaterThanOrEqual => "Greater than or equal comparison",
            OpTag::And => "Logical AND",
            OpTag::Or => "Logical OR",
            OpTag::In => "Check if value is in array or object",

            OpTag::Not => "Logical NOT",
            OpTag::DNot => "Double Logical NOT",
        }
    }
}

impl From<u32> for OpTag {
    fn from(value: u32) -> Self {
        match value as u8 {
            0x00 => OpTag::Add,
            0x01 => OpTag::Sub,
            0x02 => OpTag::Mul,
            0x03 => OpTag::Div,
            0x04 => OpTag::Mod,
            0x05 => OpTag::Min,
            0x06 => OpTag::Max,

            0x10 => OpTag::Equal,
            0x11 => OpTag::NotEqual,
            0x12 => OpTag::StrictEqual,
            0x13 => OpTag::StrictNotEqual,
            0x14 => OpTag::LessThan,
            0x15 => OpTag::LessThanOrEqual,
            0x16 => OpTag::GreaterThan,
            0x17 => OpTag::GreaterThanOrEqual,

            0x20 => OpTag::And,
            0x21 => OpTag::Or,
            0x30 => OpTag::In,
            
            0x40 => OpTag::Not,
            0x41 => OpTag::DNot,
            // Default to Add if unknown (shouldn't happen in practice)
            _ => OpTag::Add,
        }
    }
}

/// Tags for function calls used in the Call opcode
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallTag {
    Map = 0x00,
    Filter = 0x01,
    Reduce = 0x02,
    All = 0x03,
    Some = 0x04,
    None = 0x05,
    Merge = 0x06,
    Cat = 0x07,
    Substring = 0x08,
    Log = 0x09,
    Missing = 0x0A,
    MissingSome = 0x0B,
}

impl CallTag {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(CallTag::Map),
            0x01 => Some(CallTag::Filter),
            0x02 => Some(CallTag::Reduce),
            0x03 => Some(CallTag::All),
            0x04 => Some(CallTag::Some),
            0x05 => Some(CallTag::None),
            0x06 => Some(CallTag::Merge),
            0x07 => Some(CallTag::Cat),
            0x08 => Some(CallTag::Substring),
            0x09 => Some(CallTag::Log),
            0x0A => Some(CallTag::Missing),
            0x0B => Some(CallTag::MissingSome),
            _ => None,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            CallTag::Map => "Map over array",
            CallTag::Filter => "Filter array",
            CallTag::Reduce => "Reduce array",
            CallTag::All => "Check if all items match",
            CallTag::Some => "Check if some items match",
            CallTag::None => "Check if no items match",
            CallTag::Merge => "Merge arrays",
            CallTag::Cat => "Concatenate strings",
            CallTag::Substring => "Extract substring",
            CallTag::Log => "Log value (debug)",
            CallTag::Missing => "Check for missing variables",
            CallTag::MissingSome => "Check if N variables are present",
        }
    }
}

/// A compiled program, containing instructions and a constant pool.
#[derive(Debug)]
pub struct Program<'a> {
    /// The bytecode instructions
    pub instructions: &'a [Instr],

    /// The constant pool containing literal values
    pub const_pool: Vec<DataValue<'a>>,
}

impl<'a> fmt::Display for Program<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Compiled Program ===\n")?;

        // Print instructions with detailed formatting
        writeln!(f, "Instructions ({} total):", self.instructions.len())?;
        writeln!(f, "{:=^80}", "")?;
        writeln!(
            f,
            "{:<4} {:<12} {:<20} {:<40}",
            "IDX", "OPCODE", "IMMEDIATE", "DESCRIPTION"
        )?;
        writeln!(f, "{:-^80}", "")?;

        for (i, instr) in self.instructions.iter().enumerate() {
            let opcode = instr.opcode();
            let imm_formatted = instr.format_immediate();

            // Get the full description based on opcode and immediate
            let description = match opcode {
                OpCode::Variadic => {
                    let tag = OpTag::from(instr.imm() >> 16);
                    tag.description()
                }
                OpCode::Call => {
                    if let Some(tag) = CallTag::from_u8(instr.imm() as u8) {
                        tag.description()
                    } else {
                        "Unknown function call"
                    }
                }
                _ => opcode.description(),
            };

            writeln!(
                f,
                "{:<4} {:<12} {:<20} {:<40}",
                i,
                format!("{:?}", opcode),
                imm_formatted,
                description
            )?;
        }

        // Print constant pool
        writeln!(f, "\nConstant Pool ({} entries):", self.const_pool.len())?;
        writeln!(f, "{:=^80}", "")?;
        writeln!(f, "{:<4} {:<70}", "IDX", "VALUE")?;
        writeln!(f, "{:-^80}", "")?;

        for (i, value) in self.const_pool.iter().enumerate() {
            let value_str = match value {
                DataValue::String(s) if s.len() > 60 => {
                    format!("String(\"{}\"..) (length: {})", &s[..57], s.len())
                }
                _ => format!("{:?}", value),
            };

            writeln!(f, "{:<4} {:<70}", i, value_str)?;
        }

        Ok(())
    }
}
