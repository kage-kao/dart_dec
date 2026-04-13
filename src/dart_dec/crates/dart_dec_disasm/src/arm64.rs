use crate::common::*;
use crate::{DisasmError, Disassembler};
use capstone::prelude::*;
use capstone::arch::arm64::Arm64OperandType;
use dart_dec_core::Architecture;

pub struct Arm64Disassembler {
    cs: Capstone,
}

// Safety: Capstone handle is only used from one thread at a time
unsafe impl Send for Arm64Disassembler {}
unsafe impl Sync for Arm64Disassembler {}

impl Arm64Disassembler {
    pub fn new() -> Result<Self, DisasmError> {
        let cs = Capstone::new()
            .arm64()
            .mode(arch::arm64::ArchMode::Arm)
            .detail(true)
            .build()
            .map_err(|e| DisasmError::CapstoneInit(e.to_string()))?;

        Ok(Self { cs })
    }
}

impl Disassembler for Arm64Disassembler {
    fn disassemble(&self, code: &[u8], base_addr: u64) -> Result<Vec<Instruction>, DisasmError> {
        let insns = self.cs.disasm_all(code, base_addr).map_err(|e| {
            DisasmError::DisassemblyFailed {
                offset: base_addr,
                reason: e.to_string(),
            }
        })?;

        let mut result = Vec::with_capacity(insns.len());

        for insn in insns.as_ref() {
            let mnemonic = insn.mnemonic().unwrap_or("").to_string();
            let op_str = insn.op_str().unwrap_or("").to_string();

            let is_branch = matches!(
                mnemonic.as_str(),
                "b" | "b.eq"
                    | "b.ne"
                    | "b.lt"
                    | "b.gt"
                    | "b.le"
                    | "b.ge"
                    | "b.hi"
                    | "b.lo"
                    | "b.hs"
                    | "b.ls"
                    | "b.mi"
                    | "b.pl"
                    | "cbz"
                    | "cbnz"
                    | "tbz"
                    | "tbnz"
            );
            let is_call = matches!(mnemonic.as_str(), "bl" | "blr");
            let is_return = mnemonic == "ret";

            let mut operands = Vec::new();
            let mut branch_target = None;

            // Extract detailed operands
            if let Ok(detail) = self.cs.insn_detail(insn) {
                if let Some(arch_detail) = detail.arch_detail().arm64() {
                    for op in arch_detail.operands() {
                        match op.op_type {
                            Arm64OperandType::Reg(reg) => {
                                let name = self
                                    .cs
                                    .reg_name(reg)
                                    .unwrap_or_default()
                                    .to_string();
                                operands.push(InstrOperand::Register(RegInfo {
                                    id: reg.0 as u16,
                                    name,
                                }));
                            }
                            Arm64OperandType::Imm(imm) => {
                                operands.push(InstrOperand::Immediate(imm));
                                if is_branch || is_call {
                                    branch_target = Some(imm as u64);
                                }
                            }
                            Arm64OperandType::Mem(mem) => {
                                let base = if mem.base().0 != 0 {
                                    Some(RegInfo {
                                        id: mem.base().0 as u16,
                                        name: self
                                            .cs
                                            .reg_name(mem.base())
                                            .unwrap_or_default()
                                            .to_string(),
                                    })
                                } else {
                                    None
                                };
                                let index = if mem.index().0 != 0 {
                                    Some(RegInfo {
                                        id: mem.index().0 as u16,
                                        name: self
                                            .cs
                                            .reg_name(mem.index())
                                            .unwrap_or_default()
                                            .to_string(),
                                    })
                                } else {
                                    None
                                };
                                operands.push(InstrOperand::Memory(MemOperand {
                                    base,
                                    index,
                                    scale: 1,
                                    disp: mem.disp() as i64,
                                }));
                            }
                            Arm64OperandType::Fp(fp) => {
                                operands.push(InstrOperand::FloatingPoint(fp));
                            }
                            _ => {}
                        }
                    }
                }
            }

            result.push(Instruction {
                address: insn.address(),
                size: insn.len() as u8,
                mnemonic,
                op_str,
                bytes: insn.bytes().to_vec(),
                operands,
                is_branch,
                is_call,
                is_return,
                branch_target,
            });
        }

        Ok(result)
    }

    fn architecture(&self) -> Architecture {
        Architecture::Arm64
    }
}
