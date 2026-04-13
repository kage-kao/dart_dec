use dart_dec_graph::*;
use crate::PatternContext;

/// Recover Stream API patterns from async* generators
pub fn recover_streams(ast: &mut AstNode, _context: &PatternContext) {
    walk_and_transform(ast);
}

fn walk_and_transform(ast: &mut AstNode) {
    match ast {
        AstNode::Block(stmts) => {
            for stmt in stmts.iter_mut() {
                walk_and_transform(stmt);
            }
            // Look for _YieldStar stub calls -> yield* expressions
            let mut i = 0;
            while i < stmts.len() {
                if is_yield_star_call(&stmts[i]) {
                    if let AstNode::ExprStatement(AstExpr::MethodCall { args, .. }) = &stmts[i] {
                        if let Some(arg) = args.first() {
                            stmts[i] = AstNode::YieldStar(arg.clone());
                        }
                    }
                } else if is_yield_call(&stmts[i]) {
                    if let AstNode::ExprStatement(AstExpr::MethodCall { args, .. }) = &stmts[i] {
                        if let Some(arg) = args.first() {
                            stmts[i] = AstNode::Yield(arg.clone());
                        }
                    }
                }
                i += 1;
            }
        }
        AstNode::If { then_body, else_body, .. } => {
            walk_and_transform(then_body);
            if let Some(e) = else_body { walk_and_transform(e); }
        }
        AstNode::While { body, .. } | AstNode::DoWhile { body, .. } => {
            walk_and_transform(body);
        }
        _ => {}
    }
}

fn is_yield_star_call(node: &AstNode) -> bool {
    matches!(node, AstNode::ExprStatement(AstExpr::MethodCall { method, .. }) if method.contains("YieldStar"))
}

fn is_yield_call(node: &AstNode) -> bool {
    matches!(node, AstNode::ExprStatement(AstExpr::MethodCall { method, .. }) if method.contains("Yield") && !method.contains("YieldStar"))
}
