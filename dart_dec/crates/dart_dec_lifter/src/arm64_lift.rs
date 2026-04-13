use crate::ir::*;
use crate::pool_resolver::PoolResolver;
use crate::stub_resolver::StubResolver;
use crate::LiftError;
use dart_dec_disasm::Instruction;
use dart_dec_snapshot::types::SnapshotAddr;
use tracing::trace;

/// Dart AOT ARM64 register assignments
const PP_REG: u16 = 27;  // X27 = Object Pool pointer
const NULL_REG: u16 = 22; // X22 = Dart null
const THR_REG: u16 = 15;  // X15 = Thread pointer
const DISPATCH_REG: u16 = 21; // X21 = dispatch table

/// Lift ARM64 instructions into IR
pub fn lift_arm64(
    instructions: &[Instruction],
    pool_resolver: &PoolResolver,
    stub_resolver: &StubResolver,
) -> Result<Vec<IR>, LiftError> {
    let mut ir_list = Vec::with_capacity(instructions.len());

    let mut i = 0;
    while i < instructions.len() {
        let insn = &instructions[i];
        let ir = lift_single_arm64(insn, instructions, i, pool_resolver, stub_resolver);
        ir_list.extend(ir);
        i += 1;
    }

    Ok(ir_list)
}

fn lift_single_arm64(
    insn: &Instruction,
    all_insns: &[Instruction],
    idx: usize,
    pool_resolver: &PoolResolver,
    stub_resolver: &StubResolver,
) -> Vec<IR> {
    let mnemonic = insn.mnemonic.as_str();

    match mnemonic {
        // Load register from memory
        "ldr" => lift_ldr(insn, pool_resolver),

        // Store register to memory
        "str" => lift_str(insn),

        // Move instruction
        "mov" | "movz" | "movk" | "movn" => lift_mov(insn),

        // Arithmetic
        "add" | "adds" => lift_binop(insn, BinOpKind::Add),
        "sub" | "subs" => lift_binop(insn, BinOpKind::Sub),
        "mul" | "madd" => lift_binop(insn, BinOpKind::Mul),
        "sdiv" | "udiv" => lift_binop(insn, BinOpKind::Div),

        // Bitwise
        "and" | "ands" => lift_binop(insn, BinOpKind::And),
        "orr" => lift_binop(insn, BinOpKind::Or),
        "eor" => lift_binop(insn, BinOpKind::Xor),
        "lsl" => lift_binop(insn, BinOpKind::Shl),
        "lsr" => lift_binop(insn, BinOpKind::Shr),
        "asr" => lift_binop(insn, BinOpKind::Asr),

        // Compare
        "cmp" | "cmn" => lift_cmp(insn),

        // Branch
        "b" => lift_unconditional_branch(insn),
        "b.eq" | "b.ne" | "b.lt" | "b.gt" | "b.le" | "b.ge" | "b.hi" | "b.lo" | "b.hs"
        | "b.ls" => lift_conditional_branch(insn),
        "cbz" | "cbnz" => lift_cbz_cbnz(insn),
        "tbz" | "tbnz" => lift_tbz_tbnz(insn),

        // Call
        "bl" => lift_bl(insn, pool_resolver, stub_resolver),
        "blr" => lift_blr(insn, all_insns, idx, pool_resolver, stub_resolver),

        // Return
        "ret" => vec![IR::Return(Some(Operand::Reg(Reg(0))))],

        // Stack operations
        "stp" | "ldp" => vec![], // Stack frame setup/teardown — skip for now

        // Nop
        "nop" => vec![],

        // Everything else
        _ => {
            trace!("Unrecognized ARM64 instruction: {} {}", insn.mnemonic, insn.op_str);
            vec![IR::Unknown {
                address: insn.address,
                raw_asm: format!("{} {}", insn.mnemonic, insn.op_str),
            }]
        }
    }
}

fn get_reg_id(insn: &Instruction, idx: usize) -> Reg {
    if let Some(op) = insn.operands.get(idx) {
        match op {
            dart_dec_disasm::InstrOperand::Register(r) => Reg(r.id),
            _ => Reg(0),
        }
    } else {
        Reg(0)
    }
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

fn lift_ldr(insn: &Instruction, pool_resolver: &PoolResolver) -> Vec<IR> {
    let dst = get_reg_id(insn, 0);

    // Check for Object Pool load pattern: LDR Xn, [X27, #offset]
    if let Some((base, disp)) = get_mem_disp(insn, 1) {
        if base.0 == PP_REG {
            // This is an Object Pool load
            let pool_offset = disp as u64;
            let resolved = pool_resolver.resolve_offset(pool_offset);
            return vec![IR::LoadPool {
                dst,
                addr: SnapshotAddr(pool_offset),
                resolved,
            }];
        }

        // Check for null register comparison pattern
        if base.0 == NULL_REG {
            return vec![IR::Assign(dst, Operand::Imm(0))]; // null
        }

        // Regular field load
        return vec![IR::LoadField {
            dst,
            base,
            offset: disp as u32,
            field_name: None,
        }];
    }

    if let Some(imm) = get_imm(insn, 1) {
        return vec![IR::Assign(dst, Operand::Imm(imm))];
    }

    vec![IR::Unknown {
        address: insn.address,
        raw_asm: format!("{} {}", insn.mnemonic, insn.op_str),
    }]
}

fn lift_str(insn: &Instruction) -> Vec<IR> {
    let src = get_reg_id(insn, 0);

    if let Some((base, disp)) = get_mem_disp(insn, 1) {
        return vec![IR::StoreField {
            base,
            offset: disp as u32,
            value: Operand::Reg(src),
            field_name: None,
        }];
    }

    vec![IR::Unknown {
        address: insn.address,
        raw_asm: format!("{} {}", insn.mnemonic, insn.op_str),
    }]
}

fn lift_mov(insn: &Instruction) -> Vec<IR> {
    let dst = get_reg_id(insn, 0);

    if let Some(imm) = get_imm(insn, 1) {
        return vec![IR::Assign(dst, Operand::Imm(imm))];
    }

    let src = get_reg_id(insn, 1);
    vec![IR::Assign(dst, Operand::Reg(src))]
}

fn lift_binop(insn: &Instruction, op: BinOpKind) -> Vec<IR> {
    let dst = get_reg_id(insn, 0);
    let lhs = Operand::Reg(get_reg_id(insn, 1));

    let rhs = if let Some(imm) = get_imm(insn, 2) {
        Operand::Imm(imm)
    } else {
        Operand::Reg(get_reg_id(insn, 2))
    };

    vec![IR::BinOp { dst, op, lhs, rhs }]
}

fn lift_cmp(insn: &Instruction) -> Vec<IR> {
    let lhs = Operand::Reg(get_reg_id(insn, 0));
    let rhs = if let Some(imm) = get_imm(insn, 1) {
        Operand::Imm(imm)
    } else {
        Operand::Reg(get_reg_id(insn, 1))
    };

    // Check for null check pattern: CMP Xn, X22
    if let Operand::Reg(r) = &rhs {
        if r.0 == NULL_REG {
            if let Operand::Reg(checked) = &lhs {
                return vec![IR::NullCheck(*checked)];
            }
        }
    }

    vec![IR::Compare { lhs, rhs }]
}

fn lift_unconditional_branch(insn: &Instruction) -> Vec<IR> {
    if let Some(target) = insn.branch_target {
        vec![IR::Jump(target as BlockId)]
    } else {
        vec![IR::Unknown {
            address: insn.address,
            raw_asm: format!("{} {}", insn.mnemonic, insn.op_str),
        }]
    }
}

fn lift_conditional_branch(insn: &Instruction) -> Vec<IR> {
    let condition = match insn.mnemonic.as_str() {
        "b.eq" => Condition::Equal,
        "b.ne" => Condition::NotEqual,
        "b.lt" => Condition::LessThan,
        "b.gt" => Condition::GreaterThan,
        "b.le" => Condition::LessOrEqual,
        "b.ge" => Condition::GreaterOrEqual,
        _ => Condition::NotEqual,
    };

    let target = insn.branch_target.unwrap_or(0) as BlockId;
    let fallthrough = (insn.address + insn.size as u64) as BlockId;

    vec![IR::Branch {
        condition,
        true_target: target,
        false_target: fallthrough,
    }]
}

fn lift_cbz_cbnz(insn: &Instruction) -> Vec<IR> {
    let reg = get_reg_id(insn, 0);
    let target = insn.branch_target.unwrap_or(0) as BlockId;
    let fallthrough = (insn.address + insn.size as u64) as BlockId;

    let condition = if insn.mnemonic == "cbz" {
        Condition::False(reg) // branch if zero
    } else {
        Condition::True(reg) // branch if not zero
    };

    vec![IR::Branch {
        condition,
        true_target: target,
        false_target: fallthrough,
    }]
}

fn lift_tbz_tbnz(insn: &Instruction) -> Vec<IR> {
    let reg = get_reg_id(insn, 0);
    let target = insn.branch_target.unwrap_or(0) as BlockId;
    let fallthrough = (insn.address + insn.size as u64) as BlockId;

    let condition = if insn.mnemonic == "tbz" {
        Condition::False(reg)
    } else {
        Condition::True(reg)
    };

    vec![IR::Branch {
        condition,
        true_target: target,
        false_target: fallthrough,
    }]
}

fn lift_bl(
    insn: &Instruction,
    _pool_resolver: &PoolResolver,
    stub_resolver: &StubResolver,
) -> Vec<IR> {
    let target = insn.branch_target.unwrap_or(0);

    // Check if this is a known stub
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
        args: vec![
            Operand::Reg(Reg(0)),
            Operand::Reg(Reg(1)),
            Operand::Reg(Reg(2)),
            Operand::Reg(Reg(3)),
        ],
    }]
}

fn lift_blr(
    insn: &Instruction,
    all_insns: &[Instruction],
    idx: usize,
    pool_resolver: &PoolResolver,
    stub_resolver: &StubResolver,
) -> Vec<IR> {
    let target_reg = get_reg_id(insn, 0);

    // Look back for LDR pattern: LDR X16, [X27, #offset] / BLR X16
    if idx > 0 {
        let prev = &all_insns[idx - 1];
        if prev.mnemonic == "ldr" {
            if let Some((base, disp)) = get_mem_disp(prev, 1) {
                if base.0 == PP_REG {
                    // This is a pool-based call (possibly a stub)
                    if let Some(stub_kind) = stub_resolver.resolve_pool_offset(disp as u64) {
                        return vec![IR::Call {
                            dst: Some(Reg(0)),
                            kind: CallKind::Stub(stub_kind),
                            args: vec![Operand::Reg(Reg(0))],
                        }];
                    }

                    let resolved = pool_resolver.resolve_offset(disp as u64);
                    return vec![IR::Call {
                        dst: Some(Reg(0)),
                        kind: CallKind::Direct(SnapshotAddr(disp as u64)),
                        args: vec![Operand::Reg(Reg(0))],
                    }];
                }
            }
        }
    }

    vec![IR::Call {
        dst: Some(Reg(0)),
        kind: CallKind::Closure(target_reg),
        args: vec![Operand::Reg(Reg(0))],
    }]
}
