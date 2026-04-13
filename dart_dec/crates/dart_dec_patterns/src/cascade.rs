use dart_dec_graph::*;

/// Recover cascade operator (..) patterns.
///
/// Dart AOT compiles cascades:
/// ```dart
/// object..method1()..method2()..field = value
/// ```
/// Into a series of method calls on the same receiver where the result
/// is discarded (the original object is kept):
/// ```text
/// var tmp = object;
/// tmp.method1();
/// tmp.method2();
/// tmp.field = value;
/// // tmp is used after
/// ```
///
/// This pass detects sequences of method calls on the same variable
/// without using the return value, and converts them to cascade syntax.
pub fn recover_cascade(ast: &mut AstNode) {
    walk_cascade(ast);
}

fn walk_cascade(ast: &mut AstNode) {
    match ast {
        AstNode::Block(stmts) => {
            for stmt in stmts.iter_mut() {
                walk_cascade(stmt);
            }

            // Detect cascade pattern: sequence of method calls on same receiver
            let mut i = 0;
            while i < stmts.len() {
                // Find a variable assignment: var x = SomeConstructor()
                if let Some(var_name) = get_assigned_var(&stmts[i]) {
                    let mut cascade_calls = Vec::new();
                    let mut j = i + 1;

                    // Collect consecutive method calls on the same variable
                    while j < stmts.len() {
                        if let Some(call_expr) = extract_method_call_on(&stmts[j], &var_name) {
                            cascade_calls.push(call_expr);
                            j += 1;
                        } else {
                            break;
                        }
                    }

                    // Need at least 2 cascade calls to make it worthwhile
                    if cascade_calls.len() >= 2 {
                        let remove_count = cascade_calls.len();

                        // Get the original constructor/value expression
                        let base_expr = get_assigned_value(&stmts[i])
                            .unwrap_or(AstExpr::Variable(var_name.clone()));

                        let cascade = AstNode::VarDecl {
                            name: var_name.clone(),
                            dart_type: None,
                            value: Some(AstExpr::Cascade(
                                Box::new(base_expr),
                                cascade_calls,
                            )),
                            is_final: true,
                            is_late: false,
                        };

                        // Remove the individual method call statements
                        for _ in 0..remove_count {
                            if i + 1 < stmts.len() {
                                stmts.remove(i + 1);
                            }
                        }
                        stmts[i] = cascade;
                    }
                }
                i += 1;
            }
        }
        AstNode::If { then_body, else_body, .. } => {
            walk_cascade(then_body);
            if let Some(e) = else_body { walk_cascade(e); }
        }
        AstNode::While { body, .. } | AstNode::DoWhile { body, .. } | AstNode::For { body, .. } => {
            walk_cascade(body);
        }
        AstNode::TryCatch { try_body, catches, finally_body } => {
            walk_cascade(try_body);
            for c in catches { walk_cascade(&mut c.body); }
            if let Some(f) = finally_body { walk_cascade(f); }
        }
        _ => {}
    }
}

fn get_assigned_var(node: &AstNode) -> Option<String> {
    if let AstNode::VarDecl { name, value: Some(_), .. } = node {
        Some(name.clone())
    } else {
        None
    }
}

fn get_assigned_value(node: &AstNode) -> Option<AstExpr> {
    if let AstNode::VarDecl { value: Some(expr), .. } = node {
        Some(expr.clone())
    } else {
        None
    }
}

/// Extract a method call expression if it's called on the given variable
fn extract_method_call_on(node: &AstNode, target_var: &str) -> Option<AstExpr> {
    match node {
        // receiver.method(args)
        AstNode::ExprStatement(AstExpr::MethodCall {
            receiver: Some(recv),
            method,
            args,
            type_args,
        }) => {
            if let AstExpr::Variable(name) = recv.as_ref() {
                if name == target_var {
                    return Some(AstExpr::MethodCall {
                        receiver: None, // Cascade doesn't repeat receiver
                        method: method.clone(),
                        args: args.clone(),
                        type_args: type_args.clone(),
                    });
                }
            }
            None
        }
        // receiver.field = value (field assignment on the variable)
        AstNode::ExprStatement(AstExpr::BinaryOp(lhs, BinOp::Assign, rhs)) => {
            if let AstExpr::FieldAccess(base, field) = lhs.as_ref() {
                if let AstExpr::Variable(name) = base.as_ref() {
                    if name == target_var {
                        return Some(AstExpr::BinaryOp(
                            Box::new(AstExpr::FieldAccess(
                                Box::new(AstExpr::Variable(String::new())), // cascade omits receiver
                                field.clone(),
                            )),
                            BinOp::Assign,
                            rhs.clone(),
                        ));
                    }
                }
            }
            None
        }
        _ => None,
    }
}
