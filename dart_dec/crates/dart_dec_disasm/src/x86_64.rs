use crate::common::*;
use crate::{DisasmError, Disassembler};
use capstone::prelude::*;
use capstone::arch::x86::X86OperandType;
use dart_dec_core::Architecture;

pub struct X86_64Disassembler {
    cs: Capstone,
    arch: Architecture,
}

unsafe impl Send for X86_64Disassembler {}
unsafe impl Sync for X86_64Disassembler {}

impl X86_64Disassembler {
    pub fn new(arch: Architecture) -> Result<Self, DisasmError> {
        let mode = if arch == Architecture::X86 {
            arch::x86::ArchMode::Mode32
        } else {
            arch::x86::ArchMode::Mode64
        };

        let cs = Capstone::new()
            .x86()
            .mode(mode)
            .detail(true)
            .build()
            .map_err(|e| DisasmError::CapstoneInit(e.to_string()))?;

        Ok(Self { cs, arch })
    }
}

impl Disassembler for X86_64Disassembler {
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
                "jmp" | "je"
                    | "jne"
                    | "jl"
                    | "jle"
                    | "jg"
                    | "jge"
                    | "ja"
                    | "jae"
                    | "jb"
                    | "jbe"
                    | "jo"
                    | "jno"
                    | "js"
                    | "jns"
                    | "jz"
                    | "jnz"
            );
            let is_call = mnemonic == "call";
            let is_return = mnemonic == "ret" || mnemonic == "retf";

            let mut operands = Vec::new();
            let mut branch_target = None;

            if let Ok(detail) = self.cs.insn_detail(insn) {
                if let Some(arch_detail) = detail.arch_detail().x86() {
                    for op in arch_detail.operands() {
                        match op.op_type {
                            X86OperandType::Reg(reg) => {
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
                            X86OperandType::Imm(imm) => {
                                operands.push(InstrOperand::Immediate(imm));
                                if is_branch || is_call {
                                    branch_target = Some(imm as u64);
                                }
                            }
                            X86OperandType::Mem(mem) => {
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
                                    disp: mem.disp(),
                                }));
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
        self.arch
    }
}
