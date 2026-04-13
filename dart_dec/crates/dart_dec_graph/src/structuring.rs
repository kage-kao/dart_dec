use crate::ast::*;
use crate::cfg::{CFG, EdgeKind};
use crate::dominators::DominatorTree;
use crate::loop_detect::{detect_loops, LoopKind};
use dart_dec_lifter::{BlockId, IR, Condition, Operand, Reg};

/// Structure a CFG into high-level AST nodes
pub fn structure_cfg(cfg: &CFG, dom_tree: &DominatorTree) -> AstNode {
    let loops = detect_loops(cfg, dom_tree);

    let mut visited = std::collections::HashSet::new();
    let entry_id = cfg.graph[cfg.entry].id;

    structure_block(cfg, dom_tree, &loops, entry_id, &mut visited)
}

fn structure_block(
    cfg: &CFG,
    dom_tree: &DominatorTree,
    loops: &[crate::loop_detect::LoopInfo],
    block_id: BlockId,
    visited: &mut std::collections::HashSet<BlockId>,
) -> AstNode {
    if visited.contains(&block_id) {
        return AstNode::Block(vec![]);
    }
    visited.insert(block_id);

    let block = match cfg.block(block_id) {
        Some(b) => b,
        None => return AstNode::Block(vec![]),
    };

    // Check if this block is a loop header
    if let Some(loop_info) = loops.iter().find(|l| l.header == block_id) {
        return structure_loop(cfg, dom_tree, loops, loop_info, visited);
    }

    let mut stmts = Vec::new();

    // Convert IR to AST statements
    for ir in &block.instructions {
        if let Some(stmt) = ir_to_ast_stmt(ir) {
            stmts.push(stmt);
        }
    }

    // Handle control flow
    if let Some(last) = block.instructions.last() {
        match last {
            IR::Branch { condition, true_target, false_target, .. } => {
                let cond_expr = condition_to_expr(condition);
                let then_body = structure_block(cfg, dom_tree, loops, *true_target, visited);
                let else_body = if !visited.contains(false_target) {
                    Some(Box::new(structure_block(cfg, dom_tree, loops, *false_target, visited)))
                } else {
                    None
                };

                stmts.push(AstNode::If {
                    condition: cond_expr,
                    then_body: Box::new(then_body),
                    else_body,
                });
            }
            IR::Jump(target) => {
                let next = structure_block(cfg, dom_tree, loops, *target, visited);
                if let AstNode::Block(inner) = next {
                    stmts.extend(inner);
                } else {
                    stmts.push(next);
                }
            }
            IR::Return(val) => {
                let ret_expr = val.as_ref().map(operand_to_expr);
                stmts.push(AstNode::Return(ret_expr));
            }
            IR::Throw(val) => {
                stmts.push(AstNode::Throw(operand_to_expr(val)));
            }
            _ => {
                // Fallthrough to successor
                for &succ in &block.successors {
                    if !visited.contains(&succ) {
                        let next = structure_block(cfg, dom_tree, loops, succ, visited);
                        if let AstNode::Block(inner) = next {
                            stmts.extend(inner);
                        } else {
                            stmts.push(next);
                        }
                    }
                }
            }
        }
    }

    if stmts.len() == 1 {
        stmts.into_iter().next().unwrap()
    } else {
        AstNode::Block(stmts)
    }
}

fn structure_loop(
    cfg: &CFG,
    dom_tree: &DominatorTree,
    loops: &[crate::loop_detect::LoopInfo],
    loop_info: &crate::loop_detect::LoopInfo,
    visited: &mut std::collections::HashSet<BlockId>,
) -> AstNode {
    let header = cfg.block(loop_info.header).unwrap();

    match loop_info.kind {
        LoopKind::While => {
            let condition = header.instructions.last().and_then(|ir| {
                if let IR::Branch { condition, .. } = ir {
                    Some(condition_to_expr(condition))
                } else {
                    None
                }
            }).unwrap_or(AstExpr::Literal(DartLiteral::Bool(true)));

            // Mark header as visited, then structure body
            visited.insert(loop_info.header);
            let body_blocks: Vec<BlockId> = loop_info.body.iter()
                .filter(|&&b| b != loop_info.header)
                .copied()
                .collect();

            let mut body_stmts = Vec::new();
            for &bid in &body_blocks {
                if !visited.contains(&bid) {
                    let stmt = structure_block(cfg, dom_tree, loops, bid, visited);
                    body_stmts.push(stmt);
                }
            }

            AstNode::While {
                condition,
                body: Box::new(AstNode::Block(body_stmts)),
            }
        }

        LoopKind::DoWhile => {
            visited.insert(loop_info.header);
            let mut body_stmts = Vec::new();
            for &bid in &loop_info.body {
                if !visited.contains(&bid) {
                    let stmt = structure_block(cfg, dom_tree, loops, bid, visited);
                    body_stmts.push(stmt);
                }
            }

            let condition = AstExpr::Literal(DartLiteral::Bool(true));

            AstNode::DoWhile {
                body: Box::new(AstNode::Block(body_stmts)),
                condition,
            }
        }

        LoopKind::For | LoopKind::Infinite => {
            visited.insert(loop_info.header);
            let mut body_stmts = Vec::new();
            for &bid in &loop_info.body {
                if !visited.contains(&bid) {
                    let stmt = structure_block(cfg, dom_tree, loops, bid, visited);
                    body_stmts.push(stmt);
                }
            }

            AstNode::While {
                condition: AstExpr::Literal(DartLiteral::Bool(true)),
                body: Box::new(AstNode::Block(body_stmts)),
            }
        }
    }
}

fn ir_to_ast_stmt(ir: &IR) -> Option<AstNode> {
    match ir {
        IR::Assign(reg, operand) => Some(AstNode::ExprStatement(AstExpr::BinaryOp(
            Box::new(AstExpr::Variable(format!("v{}", reg.0))),
            BinOp::Assign,
            Box::new(operand_to_expr(operand)),
        ))),

        IR::Call { dst, kind, args } => {
            let call_expr = call_to_expr(kind, args);
            if let Some(d) = dst {
                Some(AstNode::VarDecl {
                    name: format!("v{}", d.0),
                    dart_type: None,
                    value: Some(call_expr),
                    is_final: false,
                    is_late: false,
                })
            } else {
                Some(AstNode::ExprStatement(call_expr))
            }
        }

        IR::NullCheck(reg) => Some(AstNode::ExprStatement(AstExpr::NullAssert(Box::new(
            AstExpr::Variable(format!("v{}", reg.0)),
        )))),

        IR::StoreField { base, offset, value, field_name } => {
            let default_field = format!("f{}", offset);
            let field = field_name.as_deref().unwrap_or(&default_field);
            Some(AstNode::ExprStatement(AstExpr::BinaryOp(
                Box::new(AstExpr::FieldAccess(
                    Box::new(AstExpr::Variable(format!("v{}", base.0))),
                    field.to_string(),
                )),
                BinOp::Assign,
                Box::new(operand_to_expr(value)),
            )))
        }

        IR::LoadField { dst, base, offset, field_name } => {
            let default_field = format!("f{}", offset);
            let field = field_name.as_deref().unwrap_or(&default_field);
            Some(AstNode::VarDecl {
                name: format!("v{}", dst.0),
                dart_type: None,
                value: Some(AstExpr::FieldAccess(
                    Box::new(AstExpr::Variable(format!("v{}", base.0))),
                    field.to_string(),
                )),
                is_final: false,
                is_late: false,
            })
        }

        IR::LoadPool { dst, resolved, .. } => {
            let expr = if let Some(name) = resolved {
                AstExpr::Variable(name.clone())
            } else {
                AstExpr::Literal(DartLiteral::Null)
            };
            Some(AstNode::VarDecl {
                name: format!("v{}", dst.0),
                dart_type: None,
                value: Some(expr),
                is_final: true,
                is_late: false,
            })
        }

        IR::BinOp { dst, op, lhs, rhs } => {
            let bin_op = match op {
                dart_dec_lifter::BinOpKind::Add => BinOp::Add,
                dart_dec_lifter::BinOpKind::Sub => BinOp::Sub,
                dart_dec_lifter::BinOpKind::Mul => BinOp::Mul,
                dart_dec_lifter::BinOpKind::Div => BinOp::Div,
                dart_dec_lifter::BinOpKind::Mod => BinOp::Mod,
                dart_dec_lifter::BinOpKind::And => BinOp::BitwiseAnd,
                dart_dec_lifter::BinOpKind::Or => BinOp::BitwiseOr,
                dart_dec_lifter::BinOpKind::Xor => BinOp::BitwiseXor,
                dart_dec_lifter::BinOpKind::Shl => BinOp::ShiftLeft,
                dart_dec_lifter::BinOpKind::Shr => BinOp::ShiftRight,
                dart_dec_lifter::BinOpKind::Asr => BinOp::ShiftRight,
            };

            Some(AstNode::VarDecl {
                name: format!("v{}", dst.0),
                dart_type: None,
                value: Some(AstExpr::BinaryOp(
                    Box::new(operand_to_expr(lhs)),
                    bin_op,
                    Box::new(operand_to_expr(rhs)),
                )),
                is_final: false,
                is_late: false,
            })
        }

        // Skip terminators — handled in structure_block
        IR::Branch { .. } | IR::Jump(_) | IR::Return(_) | IR::Throw(_) => None,
        IR::Compare { .. } => None,
        IR::Phi { .. } => None,
        IR::Unknown { raw_asm, .. } => Some(AstNode::ExprStatement(AstExpr::Variable(
            format!("/* {} */", raw_asm),
        ))),

        _ => None,
    }
}

fn operand_to_expr(op: &Operand) -> AstExpr {
    match op {
        Operand::Reg(r) => AstExpr::Variable(format!("v{}", r.0)),
        Operand::Imm(v) => AstExpr::Literal(DartLiteral::Int(*v)),
        Operand::PoolObject(addr) => AstExpr::Variable(format!("pool@{}", addr)),
        Operand::StackSlot(off) => AstExpr::Variable(format!("sp[{}]", off)),
        Operand::FieldAccess { base, offset, field_name } => {
            let default_field = format!("f{}", offset);
            let field = field_name.as_deref().unwrap_or(&default_field);
            AstExpr::FieldAccess(
                Box::new(AstExpr::Variable(format!("v{}", base.0))),
                field.to_string(),
            )
        }
    }
}

fn condition_to_expr(cond: &Condition) -> AstExpr {
    match cond {
        Condition::Equal => AstExpr::BinaryOp(
            Box::new(AstExpr::Variable("_cmp_lhs".into())),
            BinOp::Equal,
            Box::new(AstExpr::Variable("_cmp_rhs".into())),
        ),
        Condition::NotEqual => AstExpr::BinaryOp(
            Box::new(AstExpr::Variable("_cmp_lhs".into())),
            BinOp::NotEqual,
            Box::new(AstExpr::Variable("_cmp_rhs".into())),
        ),
        Condition::LessThan => AstExpr::BinaryOp(
            Box::new(AstExpr::Variable("_cmp_lhs".into())),
            BinOp::LessThan,
            Box::new(AstExpr::Variable("_cmp_rhs".into())),
        ),
        Condition::GreaterThan => AstExpr::BinaryOp(
            Box::new(AstExpr::Variable("_cmp_lhs".into())),
            BinOp::GreaterThan,
            Box::new(AstExpr::Variable("_cmp_rhs".into())),
        ),
        Condition::LessOrEqual => AstExpr::BinaryOp(
            Box::new(AstExpr::Variable("_cmp_lhs".into())),
            BinOp::LessOrEqual,
            Box::new(AstExpr::Variable("_cmp_rhs".into())),
        ),
        Condition::GreaterOrEqual => AstExpr::BinaryOp(
            Box::new(AstExpr::Variable("_cmp_lhs".into())),
            BinOp::GreaterOrEqual,
            Box::new(AstExpr::Variable("_cmp_rhs".into())),
        ),
        Condition::True(r) => AstExpr::Variable(format!("v{}", r.0)),
        Condition::False(r) => AstExpr::UnaryOp(
            UnaryOp::Not,
            Box::new(AstExpr::Variable(format!("v{}", r.0))),
        ),
        Condition::TypeCheck(r, t) => AstExpr::IsCheck(
            Box::new(AstExpr::Variable(format!("v{}", r.0))),
            t.name.clone(),
        ),
    }
}

fn call_to_expr(kind: &dart_dec_lifter::CallKind, args: &[Operand]) -> AstExpr {
    let method = match kind {
        dart_dec_lifter::CallKind::Direct(addr) => format!("func@{}", addr),
        dart_dec_lifter::CallKind::Virtual(vtable) => format!("vtable[{}]", vtable),
        dart_dec_lifter::CallKind::DynamicDispatch(name) => name.clone(),
        dart_dec_lifter::CallKind::Stub(kind) => format!("{:?}", kind),
        dart_dec_lifter::CallKind::Closure(reg) => format!("v{}()", reg.0),
    };

    AstExpr::MethodCall {
        receiver: None,
        method,
        args: args.iter().map(operand_to_expr).collect(),
        type_args: vec![],
    }
}
