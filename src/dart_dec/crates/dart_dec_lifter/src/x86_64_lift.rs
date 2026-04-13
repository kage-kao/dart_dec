use crate::ir::*;
use crate::pool_resolver::PoolResolver;
use crate::stub_resolver::StubResolver;
use crate::LiftError;
use dart_dec_disasm::Instruction;
use dart_dec_snapshot::types::SnapshotAddr;

/// Lift x86_64 instructions into IR
pub fn lift_x86_64(
    instructions: &[Instruction],
    pool_resolver: &PoolResolver,
    stub_resolver: &StubResolver,
) -> Result<Vec<IR>, LiftError> {
    let mut ir_list = Vec::with_capacity(instructions.len());

    for (i, insn) in instructions.iter().enumerate() {
        let ir = lift_single_x86(insn, instructions, i, pool_resolver, stub_resolver);
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

fn get_mem(insn: &Instruction, idx: usize) -> Option<(Reg, i64)> {
    insn.operands.get(idx).and_then(|op| match op {
        dart_dec_disasm::InstrOperand::Memory(m) => {
            let base = m.base.as_ref().map(|r| Reg(r.id)).unwrap_or(Reg(0));
            Some((base, m.disp))
        }
        _ => None,
    })
}

fn lift_single_x86(
    insn: &Instruction,
    _all_insns: &[Instruction],
    _idx: usize,
    pool_resolver: &PoolResolver,
    stub_resolver: &StubResolver,
) -> Vec<IR> {
    match insn.mnemonic.as_str() {
        "mov" | "movabs" | "movzx" | "movsx" | "movsxd" => {
            let dst = get_reg_id(insn, 0);
            if let Some(imm) = get_imm(insn, 1) {
                vec![IR::Assign(dst, Operand::Imm(imm))]
            } else if let Some((base, disp)) = get_mem(insn, 1) {
                vec![IR::LoadField { dst, base, offset: disp as u32, field_name: None }]
            } else {
                let src = get_reg_id(insn, 1);
                vec![IR::Assign(dst, Operand::Reg(src))]
            }
        }

        "lea" => {
            let dst = get_reg_id(insn, 0);
            if let Some((base, disp)) = get_mem(insn, 1) {
                vec![IR::BinOp {
                    dst,
                    op: BinOpKind::Add,
                    lhs: Operand::Reg(base),
                    rhs: Operand::Imm(disp),
                }]
            } else {
                vec![IR::Unknown { address: insn.address, raw_asm: format!("{} {}", insn.mnemonic, insn.op_str) }]
            }
        }

        "add" => {
            let dst = get_reg_id(insn, 0);
            let rhs = get_imm(insn, 1).map(Operand::Imm).unwrap_or(Operand::Reg(get_reg_id(insn, 1)));
            vec![IR::BinOp { dst, op: BinOpKind::Add, lhs: Operand::Reg(dst), rhs }]
        }

        "sub" => {
            let dst = get_reg_id(insn, 0);
            let rhs = get_imm(insn, 1).map(Operand::Imm).unwrap_or(Operand::Reg(get_reg_id(insn, 1)));
            vec![IR::BinOp { dst, op: BinOpKind::Sub, lhs: Operand::Reg(dst), rhs }]
        }

        "imul" => {
            let dst = get_reg_id(insn, 0);
            let rhs = get_imm(insn, 2).map(Operand::Imm).unwrap_or(Operand::Reg(get_reg_id(insn, 1)));
            vec![IR::BinOp { dst, op: BinOpKind::Mul, lhs: Operand::Reg(get_reg_id(insn, 1)), rhs }]
        }

        "and" => {
            let dst = get_reg_id(insn, 0);
            let rhs = get_imm(insn, 1).map(Operand::Imm).unwrap_or(Operand::Reg(get_reg_id(insn, 1)));
            vec![IR::BinOp { dst, op: BinOpKind::And, lhs: Operand::Reg(dst), rhs }]
        }

        "or" => {
            let dst = get_reg_id(insn, 0);
            let rhs = get_imm(insn, 1).map(Operand::Imm).unwrap_or(Operand::Reg(get_reg_id(insn, 1)));
            vec![IR::BinOp { dst, op: BinOpKind::Or, lhs: Operand::Reg(dst), rhs }]
        }

        "xor" => {
            let dst = get_reg_id(insn, 0);
            let rhs = get_imm(insn, 1).map(Operand::Imm).unwrap_or(Operand::Reg(get_reg_id(insn, 1)));
            vec![IR::BinOp { dst, op: BinOpKind::Xor, lhs: Operand::Reg(dst), rhs }]
        }

        "shl" | "sal" => {
            let dst = get_reg_id(insn, 0);
            let rhs = get_imm(insn, 1).map(Operand::Imm).unwrap_or(Operand::Reg(get_reg_id(insn, 1)));
            vec![IR::BinOp { dst, op: BinOpKind::Shl, lhs: Operand::Reg(dst), rhs }]
        }

        "shr" => {
            let dst = get_reg_id(insn, 0);
            let rhs = get_imm(insn, 1).map(Operand::Imm).unwrap_or(Operand::Reg(get_reg_id(insn, 1)));
            vec![IR::BinOp { dst, op: BinOpKind::Shr, lhs: Operand::Reg(dst), rhs }]
        }

        "cmp" | "test" => {
            let lhs = Operand::Reg(get_reg_id(insn, 0));
            let rhs = get_imm(insn, 1).map(Operand::Imm).unwrap_or(Operand::Reg(get_reg_id(insn, 1)));
            vec![IR::Compare { lhs, rhs }]
        }

        "jmp" => {
            if let Some(target) = insn.branch_target {
                vec![IR::Jump(target as BlockId)]
            } else {
                vec![IR::Unknown { address: insn.address, raw_asm: format!("{} {}", insn.mnemonic, insn.op_str) }]
            }
        }

        "je" | "jz" => branch_ir(insn, Condition::Equal),
        "jne" | "jnz" => branch_ir(insn, Condition::NotEqual),
        "jl" | "jnge" => branch_ir(insn, Condition::LessThan),
        "jg" | "jnle" => branch_ir(insn, Condition::GreaterThan),
        "jle" | "jng" => branch_ir(insn, Condition::LessOrEqual),
        "jge" | "jnl" => branch_ir(insn, Condition::GreaterOrEqual),

        "call" => {
            let target = insn.branch_target.unwrap_or(0);
            if let Some(stub_kind) = stub_resolver.resolve_address(target) {
                return vec![IR::Call {
                    dst: Some(Reg(0)),
                    kind: CallKind::Stub(stub_kind),
                    args: vec![],
                }];
            }
            vec![IR::Call {
                dst: Some(Reg(0)),
                kind: CallKind::Direct(SnapshotAddr(target)),
                args: vec![],
            }]
        }

        "ret" | "retf" => vec![IR::Return(Some(Operand::Reg(Reg(0))))],

        "push" | "pop" | "nop" | "endbr64" => vec![],

        _ => vec![IR::Unknown {
            address: insn.address,
            raw_asm: format!("{} {}", insn.mnemonic, insn.op_str),
        }],
    }
}

fn branch_ir(insn: &Instruction, condition: Condition) -> Vec<IR> {
    let target = insn.branch_target.unwrap_or(0) as BlockId;
    let fallthrough = (insn.address + insn.size as u64) as BlockId;
    vec![IR::Branch { condition, true_target: target, false_target: fallthrough }]
}
