//! dart_dec_lifter — IR lifter: translates disassembled instructions into
//! platform-independent Intermediate Representation (IR).

mod arm32_lift;
mod arm64_lift;
mod ir;
mod pool_resolver;
mod stub_resolver;
mod x86_64_lift;

pub use ir::*;
pub use pool_resolver::PoolResolver;
pub use stub_resolver::StubResolver;

use dart_dec_core::Architecture;
use dart_dec_disasm::{DisasmError, Instruction};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LiftError {
    #[error("Unsupported architecture for lifting: {0}")]
    UnsupportedArch(String),

    #[error("Failed to lift instruction at 0x{addr:x}: {reason}")]
    LiftFailed { addr: u64, reason: String },

    #[error("Disassembly error: {0}")]
    Disasm(#[from] DisasmError),
}

/// Lift a sequence of instructions into IR
pub fn lift_instructions(
    instructions: &[Instruction],
    arch: Architecture,
    pool_resolver: &PoolResolver,
    stub_resolver: &StubResolver,
) -> Result<Vec<IR>, LiftError> {
    match arch {
        Architecture::Arm64 => arm64_lift::lift_arm64(instructions, pool_resolver, stub_resolver),
        Architecture::Arm32 => arm32_lift::lift_arm32(instructions, pool_resolver, stub_resolver),
        Architecture::X86_64 | Architecture::X86 => {
            x86_64_lift::lift_x86_64(instructions, pool_resolver, stub_resolver)
        }
    }
}

#[cfg(test)]
mod tests;
