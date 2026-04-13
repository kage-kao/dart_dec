use serde::Serialize;

/// A single disassembled instruction
#[derive(Debug, Clone, Serialize)]
pub struct Instruction {
    pub address: u64,
    pub size: u8,
    pub mnemonic: String,
    pub op_str: String,
    pub bytes: Vec<u8>,
    pub operands: Vec<InstrOperand>,
    pub is_branch: bool,
    pub is_call: bool,
    pub is_return: bool,
    pub branch_target: Option<u64>,
}

/// Operand of an instruction
#[derive(Debug, Clone, Serialize)]
pub enum InstrOperand {
    Register(RegInfo),
    Immediate(i64),
    Memory(MemOperand),
    FloatingPoint(f64),
}

/// Register information
#[derive(Debug, Clone, Serialize)]
pub struct RegInfo {
    pub id: u16,
    pub name: String,
}

/// Memory operand (base + index*scale + disp)
#[derive(Debug, Clone, Serialize)]
pub struct MemOperand {
    pub base: Option<RegInfo>,
    pub index: Option<RegInfo>,
    pub scale: i32,
    pub disp: i64,
}

impl Instruction {
    /// Check if this instruction is a branch/jump
    pub fn is_control_flow(&self) -> bool {
        self.is_branch || self.is_call || self.is_return
    }

    /// Get a human-readable representation
    pub fn to_string_repr(&self) -> String {
        format!("0x{:08x}: {} {}", self.address, self.mnemonic, self.op_str)
    }
}
