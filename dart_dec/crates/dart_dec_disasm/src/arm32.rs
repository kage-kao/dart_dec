use crate::common::*;
use crate::{DisasmError, Disassembler};
use capstone::prelude::*;
use capstone::arch::arm::ArmOperandType;
use dart_dec_core::Architecture;

pub struct Arm32Disassembler {
    cs: Capstone,
}

unsafe impl Send for Arm32Disassembler {}
unsafe impl Sync for Arm32Disassembler {}

impl Arm32Disassembler {
    pub fn new() -> Result<Self, DisasmError> {
        let cs = Capstone::new()
            .arm()
            .mode(arch::arm::ArchMode::Thumb)
            .detail(true)
            .build()
            .map_err(|e| DisasmError::CapstoneInit(e.to_string()))?;

        Ok(Self { cs })
    }
}

impl Disassembler for Arm32Disassembler {
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
                "b" | "beq"
                    | "bne"
                    | "blt"
                    | "bgt"
                    | "ble"
                    | "bge"
                    | "bhi"
                    | "blo"
                    | "bhs"
                    | "bls"
                    | "cbz"
                    | "cbnz"
            );
            let is_call = matches!(mnemonic.as_str(), "bl" | "blx");
            let is_return = mnemonic == "bx" && op_str.contains("lr");

            let mut operands = Vec::new();
            let mut branch_target = None;

            if let Ok(detail) = self.cs.insn_detail(insn) {
                if let Some(arch_detail) = detail.arch_detail().arm() {
                    for op in arch_detail.operands() {
                        match op.op_type {
                            ArmOperandType::Reg(reg) => {
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
                            ArmOperandType::Imm(imm) => {
                                operands.push(InstrOperand::Immediate(imm as i64));
                                if is_branch || is_call {
                                    branch_target = Some(imm as u64);
                                }
                            }
                            ArmOperandType::Mem(mem) => {
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
                                    scale: mem.scale(),
                                    disp: mem.disp() as i64,
                                }));
                            }
                            ArmOperandType::Fp(fp) => {
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
        Architecture::Arm32
    }
}
