use crate::ir::*;
use crate::pool_resolver::PoolResolver;
use crate::stub_resolver::StubResolver;
use crate::LiftError;
use dart_dec_disasm::Instruction;
use dart_dec_snapshot::types::SnapshotAddr;

/// Lift ARM32 (Thumb2) instructions into IR
pub fn lift_arm32(
    instructions: &[Instruction],
    pool_resolver: &PoolResolver,
    stub_resolver: &StubResolver,
) -> Result<Vec<IR>, LiftError> {
    let mut ir_list = Vec::with_capacity(instructions.len());

    for (i, insn) in instructions.iter().enumerate() {
        let ir = lift_single_arm32(insn, instructions, i, pool_resolver, stub_resolver);
        ir_list.extend(ir);
    }

    Ok(ir_list)
}

fn get_reg_id(insn: &Instruction, idx: usize) -> Reg {
    insn.operands.get(idx).map(|op| match op {
        dart_dec_disasm::InstrOperand::Register(r) => Reg(r.id),
        _ => Reg(0),
    }).unwrap_or(Reg(0))
}

fn get_imm(insn: &Instruction, idx: usize) -> Option<i64> {
    insn.operands.get(idx).and_then(|op| match op {
        dart_dec_disasm::InstrOperand::Immediate(v) => Some(*v),
        _ => None,
    })
}

fn get_mem_disp(insn: &Instruction, idx: usize) -> Option<(Reg, i64)> {
    insn.operands.get(idx).and_then(|op| match op {
        dart_dec_disasm::InstrOperand::Memory(m) => {
            let base = m.base.as_ref().map(|r| Reg(r.id)).unwrap_or(Reg(0));
            Some((base, m.disp))
        }
        _ => None,
    })
}

fn lift_single_arm32(
    insn: &Instruction,
    all_insns: &[Instruction],
    idx: usize,
    pool_resolver: &PoolResolver,
    stub_resolver: &StubResolver,
) -> Vec<IR> {
    match insn.mnemonic.as_str() {
        "ldr" | "ldr.w" => {
            let dst = get_reg_id(insn, 0);
            if let Some((base, disp)) = get_mem_disp(insn, 1) {
                // Check for pool load (ARM32 uses different PP register)
                if base.0 == 10 { // R10 typically used as PP on ARM32
                    let resolved = pool_resolver.resolve_offset(disp as u64);
                    return vec![IR::LoadPool {
                        dst,
                        addr: SnapshotAddr(disp as u64),
                        resolved,
                    }];
                }
                return vec![IR::LoadField {
                    dst,
                    base,
                    offset: disp as u32,
                    field_name: None,
                }];
            }
            vec![IR::Unknown { address: insn.address, raw_asm: format!("{} {}", insn.mnemonic, insn.op_str) }]
        }

        "str" | "str.w" => {
            let src = get_reg_id(insn, 0);
            if let Some((base, disp)) = get_mem_disp(insn, 1) {
                return vec![IR::StoreField {
                    base,
                    offset: disp as u32,
                    value: Operand::Reg(src),
                    field_name: None,
                }];
            }
            vec![IR::Unknown { address: insn.address, raw_asm: format!("{} {}", insn.mnemonic, insn.op_str) }]
        }

        "mov" | "movs" | "movw" | "movt" => {
            let dst = get_reg_id(insn, 0);
            if let Some(imm) = get_imm(insn, 1) {
                vec![IR::Assign(dst, Operand::Imm(imm))]
            } else {
                let src = get_reg_id(insn, 1);
                vec![IR::Assign(dst, Operand::Reg(src))]
            }
        }

        "add" | "adds" => {
            let dst = get_reg_id(insn, 0);
            let lhs = Operand::Reg(get_reg_id(insn, 1));
            let rhs = get_imm(insn, 2).map(Operand::Imm).unwrap_or(Operand::Reg(get_reg_id(insn, 2)));
            vec![IR::BinOp { dst, op: BinOpKind::Add, lhs, rhs }]
        }

        "sub" | "subs" => {
            let dst = get_reg_id(insn, 0);
            let lhs = Operand::Reg(get_reg_id(insn, 1));
            let rhs = get_imm(insn, 2).map(Operand::Imm).unwrap_or(Operand::Reg(get_reg_id(insn, 2)));
            vec![IR::BinOp { dst, op: BinOpKind::Sub, lhs, rhs }]
        }

        "mul" | "muls" => {
            let dst = get_reg_id(insn, 0);
            let lhs = Operand::Reg(get_reg_id(insn, 1));
            let rhs = Operand::Reg(get_reg_id(insn, 2));
            vec![IR::BinOp { dst, op: BinOpKind::Mul, lhs, rhs }]
        }

        "cmp" | "cmn" => {
            let lhs = Operand::Reg(get_reg_id(insn, 0));
            let rhs = get_imm(insn, 1).map(Operand::Imm).unwrap_or(Operand::Reg(get_reg_id(insn, 1)));
            vec![IR::Compare { lhs, rhs }]
        }

        "b" | "b.w" => {
            if let Some(target) = insn.branch_target {
                vec![IR::Jump(target as BlockId)]
            } else {
                vec![IR::Unknown { address: insn.address, raw_asm: format!("{} {}", insn.mnemonic, insn.op_str) }]
            }
        }

        "beq" | "bne" | "blt" | "bgt" | "ble" | "bge" => {
            let condition = match insn.mnemonic.as_str() {
                "beq" => Condition::Equal,
                "bne" => Condition::NotEqual,
                "blt" => Condition::LessThan,
                "bgt" => Condition::GreaterThan,
                "ble" => Condition::LessOrEqual,
                "bge" => Condition::GreaterOrEqual,
                _ => Condition::NotEqual,
            };
            let target = insn.branch_target.unwrap_or(0) as BlockId;
            let fallthrough = (insn.address + insn.size as u64) as BlockId;
            vec![IR::Branch { condition, true_target: target, false_target: fallthrough }]
        }

        "bl" | "blx" => {
            let target = insn.branch_target.unwrap_or(0);
            if let Some(stub_kind) = stub_resolver.resolve_address(target) {
                return vec![IR::Call {
                    dst: Some(Reg(0)),
                    kind: CallKind::Stub(stub_kind),
                    args: vec![Operand::Reg(Reg(0))],
                }];
            }
            vec![IR::Call {
                dst: Some(Reg(0)),
                kind: CallKind::Direct(SnapshotAddr(target)),
                args: vec![Operand::Reg(Reg(0)), Operand::Reg(Reg(1)), Operand::Reg(Reg(2)), Operand::Reg(Reg(3))],
            }]
        }

        "bx" if insn.op_str.contains("lr") => vec![IR::Return(Some(Operand::Reg(Reg(0))))],
        "pop" if insn.op_str.contains("pc") => vec![IR::Return(Some(Operand::Reg(Reg(0))))],

        "push" | "pop" | "nop" => vec![],

        _ => vec![IR::Unknown {
            address: insn.address,
            raw_asm: format!("{} {}", insn.mnemonic, insn.op_str),
        }],
    }
}
