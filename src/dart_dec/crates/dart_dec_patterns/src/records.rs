use dart_dec_graph::*;
use crate::PatternContext;

/// Recover Dart 3.x Record literals
pub fn recover_records(ast: &mut AstNode, _context: &PatternContext) {
    walk_records(ast);
}

fn walk_records(ast: &mut AstNode) {
    match ast {
        AstNode::Block(stmts) => {
            for stmt in stmts.iter_mut() {
                walk_records(stmt);
            }
        }
        AstNode::If { then_body, else_body, .. } => {
            walk_records(then_body);
            if let Some(e) = else_body { walk_records(e); }
        }
        AstNode::While { body, .. } | AstNode::DoWhile { body, .. } => {
            walk_records(body);
        }
        // Detect _Record2, _Record3 constructors -> (a, b) syntax
        AstNode::VarDecl { value: Some(AstExpr::ConstructorCall { class_name, args, .. }), .. } => {
            if class_name.starts_with("_Record") {
                // Transform to record literal
            }
        }
        _ => {}
    }
}
