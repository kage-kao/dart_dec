use dart_dec_graph::*;

/// Recover extension method patterns.
///
/// In Dart AOT, extension methods are compiled as static functions
/// with the receiver as the first argument. The pattern is:
/// - Static call: ExtensionName.methodName(receiver, args...)
/// - Needs to be recovered to: receiver.methodName(args...)
///
/// Detection heuristics:
/// 1. Function name contains the extension class prefix
/// 2. First parameter type matches the 'on' type of the extension
/// 3. Call sites use static dispatch instead of virtual
pub fn recover_extensions(ast: &mut AstNode) {
    walk_extensions(ast);
}

fn walk_extensions(ast: &mut AstNode) {
    match ast {
        AstNode::Block(stmts) => {
            for stmt in stmts.iter_mut() {
                walk_extensions(stmt);
                transform_extension_calls(stmt);
            }
        }
        AstNode::If { then_body, else_body, .. } => {
            walk_extensions(then_body);
            if let Some(e) = else_body {
                walk_extensions(e);
            }
        }
        AstNode::While { body, .. } | AstNode::DoWhile { body, .. } | AstNode::For { body, .. } => {
            walk_extensions(body);
        }
        AstNode::TryCatch { try_body, catches, finally_body } => {
            walk_extensions(try_body);
            for c in catches {
                walk_extensions(&mut c.body);
            }
            if let Some(f) = finally_body {
                walk_extensions(f);
            }
        }
        _ => {}
    }
}

/// Transform static extension calls to method-style calls.
/// Pattern: ExtName|methodName(receiver, arg1, arg2)
/// => receiver.methodName(arg1, arg2)
fn transform_extension_calls(node: &mut AstNode) {
    match node {
        AstNode::ExprStatement(expr) => {
            transform_expr_extension(expr);
        }
        AstNode::VarDecl { value: Some(expr), .. } => {
            transform_expr_extension(expr);
        }
        _ => {}
    }
}

fn transform_expr_extension(expr: &mut AstExpr) {
    match expr {
        AstExpr::MethodCall {
            receiver,
            method,
            args,
            type_args,
        } => {
            // Check for ExtensionName|methodName pattern (Dart AOT uses | separator)
            if receiver.is_none() && method.contains('|') {
                let parts: Vec<&str> = method.splitn(2, '|').collect();
                if parts.len() == 2 && !args.is_empty() {
                    let real_method = parts[1].to_string();
                    let real_receiver = args[0].clone();
                    let real_args = args[1..].to_vec();

                    *receiver = Some(Box::new(real_receiver));
                    *method = real_method;
                    *args = real_args;
                }
            }

            // Check for static calls with extension prefix: _extension#0|methodName
            if receiver.is_none() && method.contains("#") && method.contains("|") {
                if let Some(pipe_pos) = method.find('|') {
                    let real_method = method[pipe_pos + 1..].to_string();
                    if !args.is_empty() {
                        let real_receiver = args[0].clone();
                        let real_args = args[1..].to_vec();

                        *receiver = Some(Box::new(real_receiver));
                        *method = real_method;
                        *args = real_args;
                    }
                }
            }

            // Recurse into sub-expressions
            if let Some(r) = receiver {
                transform_expr_extension(r);
            }
            for arg in args.iter_mut() {
                transform_expr_extension(arg);
            }
        }
        AstExpr::BinaryOp(lhs, _, rhs) => {
            transform_expr_extension(lhs);
            transform_expr_extension(rhs);
        }
        AstExpr::FieldAccess(base, _) => {
            transform_expr_extension(base);
        }
        AstExpr::NullAwareAccess(base, _) | AstExpr::NullAssert(base) => {
            transform_expr_extension(base);
        }
        AstExpr::Conditional(cond, then_e, else_e) => {
            transform_expr_extension(cond);
            transform_expr_extension(then_e);
            transform_expr_extension(else_e);
        }
        _ => {}
    }
}
