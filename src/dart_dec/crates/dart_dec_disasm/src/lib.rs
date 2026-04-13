//! dart_dec_disasm — Multi-architecture disassembler using Capstone.
//!
//! Supports ARM64, ARM32 (Thumb2), and x86_64 disassembly with
//! detailed operand information for IR lifting.

mod arm32;
mod arm64;
mod common;
mod x86_64;

pub use arm32::Arm32Disassembler;
pub use arm64::Arm64Disassembler;
pub use common::*;
pub use x86_64::X86_64Disassembler;

use dart_dec_core::Architecture;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DisasmError {
    #[error("Capstone initialization failed: {0}")]
    CapstoneInit(String),

    #[error("Disassembly failed at offset 0x{offset:x}: {reason}")]
    DisassemblyFailed { offset: u64, reason: String },

    #[error("Unsupported architecture for disassembly: {0}")]
    UnsupportedArch(String),
}

/// Create a disassembler for the given architecture
pub fn create_disassembler(arch: Architecture) -> Result<Box<dyn Disassembler>, DisasmError> {
    match arch {
        Architecture::Arm64 => Ok(Box::new(Arm64Disassembler::new()?)),
        Architecture::Arm32 => Ok(Box::new(Arm32Disassembler::new()?)),
        Architecture::X86_64 | Architecture::X86 => Ok(Box::new(X86_64Disassembler::new(arch)?)),
    }
}

/// Trait for architecture-specific disassemblers
pub trait Disassembler: Send + Sync {
    /// Disassemble a chunk of bytes into instructions
    fn disassemble(&self, code: &[u8], base_addr: u64) -> Result<Vec<Instruction>, DisasmError>;

    /// Get the architecture
    fn architecture(&self) -> Architecture;
}

#[cfg(test)]
mod tests;
