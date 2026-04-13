use dart_dec_graph::*;

/// Recover null safety operators (?, !, late)
pub fn recover_null_safety(ast: &mut AstNode) {
    walk_null(ast);
}

fn walk_null(ast: &mut AstNode) {
    match ast {
        AstNode::Block(stmts) => {
            let mut i = 0;
            while i < stmts.len() {
                walk_null(&mut stmts[i]);
                // Pattern: if (x == null) throw NullCheckError -> x!
                if i + 1 < stmts.len() {
                    if is_null_check_throw_pattern(&stmts[i], &stmts[i + 1]) {
                        // Replace with null assertion
                        if let AstNode::If { condition: AstExpr::BinaryOp(lhs_box, BinOp::Equal, rhs_box), .. } = &stmts[i] {
                            if let (AstExpr::Variable(var), AstExpr::Literal(DartLiteral::Null)) = (lhs_box.as_ref(), rhs_box.as_ref()) {
                                stmts[i] = AstNode::ExprStatement(AstExpr::NullAssert(
                                    Box::new(AstExpr::Variable(var.clone()))
                                ));
                                stmts.remove(i + 1);
                            }
                        }
                    }
                }
                i += 1;
            }
        }
        AstNode::If { then_body, else_body, .. } => {
            walk_null(then_body);
            if let Some(e) = else_body { walk_null(e); }
        }
        AstNode::While { body, .. } | AstNode::DoWhile { body, .. } => {
            walk_null(body);
        }
        _ => {}
    }
}

fn is_null_check_throw_pattern(_check: &AstNode, _throw: &AstNode) -> bool {
    // Simplified detection
    false
}
