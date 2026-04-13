use dart_dec_graph::*;

/// Recover closure/lambda expressions from AOT-compiled closures.
///
/// In Dart AOT, closures are compiled as separate functions with captured
/// context passed as an argument. The pattern is:
/// 1. AllocateClosure stub call with a function reference
/// 2. StoreField calls to fill captured variables into the context
/// 3. The closure function reads captured vars from context parameter
pub fn recover_closures(ast: &mut AstNode) {
    walk_closures(ast);
}

fn walk_closures(ast: &mut AstNode) {
    match ast {
        AstNode::Block(stmts) => {
            let mut i = 0;
            while i < stmts.len() {
                walk_closures(&mut stmts[i]);

                // Pattern: var closure = AllocateClosure(func_ref)
                // followed by: closure.context.field0 = captured_var0
                // followed by: closure.context.field1 = captured_var1
                // => Transform to: (params) => body
                if is_closure_allocation(&stmts[i]) {
                    let closure_var = extract_closure_var(&stmts[i]);
                    if let Some(var_name) = closure_var {
                        // Collect context stores
                        let mut captured_vars = Vec::new();
                        let mut j = i + 1;
                        while j < stmts.len() && is_context_store(&stmts[j], &var_name) {
                            if let Some((_field, value)) = extract_context_store(&stmts[j]) {
                                captured_vars.push(value);
                            }
                            j += 1;
                        }

                        // Generate lambda params from captured variable count
                        let params: Vec<String> = (0..captured_vars.len())
                            .map(|idx| format!("arg{}", idx))
                            .collect();

                        // Replace allocation + stores with lambda expression
                        let lambda = AstNode::VarDecl {
                            name: var_name.clone(),
                            dart_type: Some("Function".to_string()),
                            value: Some(AstExpr::Lambda {
                                params,
                                body: Box::new(AstNode::ExprStatement(
                                    AstExpr::Variable("/* closure body */".to_string()),
                                )),
                            }),
                            is_final: true,
                            is_late: false,
                        };

                        // Remove context stores and replace allocation
                        let remove_count = j - i - 1;
                        for _ in 0..remove_count {
                            if i + 1 < stmts.len() {
                                stmts.remove(i + 1);
                            }
                        }
                        stmts[i] = lambda;
                    }
                }

                // Pattern: single-expression function body => arrow function
                if let AstNode::VarDecl {
                    value: Some(AstExpr::MethodCall { method, args, .. }),
                    ..
                } = &stmts[i]
                {
                    if method.contains("closure") && args.len() == 1 {
                        // This might be a closure invocation
                    }
                }

                i += 1;
            }
        }
        AstNode::If { then_body, else_body, .. } => {
            walk_closures(then_body);
            if let Some(e) = else_body {
                walk_closures(e);
            }
        }
        AstNode::While { body, .. } | AstNode::DoWhile { body, .. } => {
            walk_closures(body);
        }
        AstNode::For { body, init, update, .. } => {
            walk_closures(body);
            walk_closures(init);
            walk_closures(update);
        }
        AstNode::TryCatch {
            try_body,
            catches,
            finally_body,
        } => {
            walk_closures(try_body);
            for catch in catches {
                walk_closures(&mut catch.body);
            }
            if let Some(f) = finally_body {
                walk_closures(f);
            }
        }
        AstNode::Switch { cases, default, .. } => {
            for (_, body) in cases.iter_mut() {
                walk_closures(body);
            }
            if let Some(d) = default {
                walk_closures(d);
            }
        }
        _ => {}
    }
}

fn is_closure_allocation(node: &AstNode) -> bool {
    match node {
        AstNode::VarDecl {
            value: Some(AstExpr::MethodCall { method, .. }),
            ..
        } => method.contains("AllocateObject") || method.contains("Closure"),
        AstNode::ExprStatement(AstExpr::MethodCall { method, .. }) => {
            method.contains("AllocateObject") || method.contains("Closure")
        }
        _ => false,
    }
}

fn extract_closure_var(node: &AstNode) -> Option<String> {
    match node {
        AstNode::VarDecl { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn is_context_store(node: &AstNode, closure_var: &str) -> bool {
    match node {
        AstNode::ExprStatement(AstExpr::BinaryOp(
            box_lhs,
            BinOp::Assign,
            _,
        )) => {
            if let AstExpr::FieldAccess(receiver, _) = box_lhs.as_ref() {
                if let AstExpr::FieldAccess(base, field) = receiver.as_ref() {
                    if let AstExpr::Variable(name) = base.as_ref() {
                        return name == closure_var && field == "context";
                    }
                }
                if let AstExpr::Variable(name) = receiver.as_ref() {
                    return name == closure_var;
                }
            }
            false
        }
        _ => false,
    }
}

fn extract_context_store(node: &AstNode) -> Option<(String, AstExpr)> {
    match node {
        AstNode::ExprStatement(AstExpr::BinaryOp(lhs, BinOp::Assign, rhs)) => {
            if let AstExpr::FieldAccess(_, field_name) = lhs.as_ref() {
                return Some((field_name.clone(), rhs.as_ref().clone()));
            }
            None
        }
        _ => None,
    }
}
