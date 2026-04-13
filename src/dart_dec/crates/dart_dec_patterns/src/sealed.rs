use dart_dec_graph::*;
use crate::PatternContext;

/// Recover sealed class exhaustive switch patterns
pub fn recover_sealed_patterns(ast: &mut AstNode, context: &PatternContext) {
    walk_sealed(ast, context);
}

fn walk_sealed(ast: &mut AstNode, context: &PatternContext) {
    match ast {
        AstNode::Block(stmts) => {
            for stmt in stmts.iter_mut() {
                walk_sealed(stmt, context);
            }
        }
        AstNode::Switch { subject, cases, default } => {
            // Check if switch covers all subclasses of a sealed class
            if let AstExpr::IsCheck(_, type_name) = subject {
                if context.sealed_classes.contains(type_name) && default.is_some() {
                    // This is an exhaustive switch — remove default
                    *default = None;
                }
            }
            for (_, body) in cases.iter_mut() {
                walk_sealed(body, context);
            }
        }
        AstNode::If { then_body, else_body, .. } => {
            walk_sealed(then_body, context);
            if let Some(e) = else_body { walk_sealed(e, context); }
        }
        _ => {}
    }
}
