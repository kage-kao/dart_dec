use dart_dec_graph::*;

/// Recover string interpolation from StringBuffer patterns.
///
/// Dart AOT compiles `"Hello, $name!"` into:
/// ```text
/// var sb = StringBuffer();
/// sb.write("Hello, ");
/// sb.write(name.toString());
/// sb.write("!");
/// var result = sb.toString();
/// ```
///
/// This pass recovers the original `"Hello, ${name}!"` form.
pub fn recover_string_interpolation(ast: &mut AstNode) {
    walk_interp(ast);
}

fn walk_interp(ast: &mut AstNode) {
    match ast {
        AstNode::Block(stmts) => {
            for stmt in stmts.iter_mut() {
                walk_interp(stmt);
            }

            // Pattern: StringBuffer alloc + write series + toString
            let mut i = 0;
            while i < stmts.len() {
                if let Some(sb_var) = detect_string_buffer_alloc(&stmts[i]) {
                    let mut parts: Vec<StringPart> = Vec::new();
                    let mut j = i + 1;

                    // Collect write() calls
                    while j < stmts.len() {
                        if let Some(part) = extract_write_call(&stmts[j], &sb_var) {
                            parts.push(part);
                            j += 1;
                        } else {
                            break;
                        }
                    }

                    // Check for toString() assignment
                    if j < stmts.len() && !parts.is_empty() {
                        if let Some(result_var) = extract_to_string(&stmts[j], &sb_var) {
                            // Replace everything with a string interpolation
                            let interp = AstNode::VarDecl {
                                name: result_var,
                                dart_type: Some("String".to_string()),
                                value: Some(AstExpr::StringInterpolation(parts)),
                                is_final: true,
                                is_late: false,
                            };

                            // Remove intermediate statements
                            let remove_count = j - i;
                            for _ in 0..remove_count {
                                if i + 1 < stmts.len() {
                                    stmts.remove(i + 1);
                                }
                            }
                            stmts[i] = interp;
                        }
                    }
                }
                i += 1;
            }
        }
        AstNode::If { then_body, else_body, .. } => {
            walk_interp(then_body);
            if let Some(e) = else_body { walk_interp(e); }
        }
        AstNode::While { body, .. } | AstNode::DoWhile { body, .. } | AstNode::For { body, .. } => {
            walk_interp(body);
        }
        AstNode::TryCatch { try_body, catches, finally_body } => {
            walk_interp(try_body);
            for c in catches { walk_interp(&mut c.body); }
            if let Some(f) = finally_body { walk_interp(f); }
        }
        _ => {}
    }
}

fn detect_string_buffer_alloc(node: &AstNode) -> Option<String> {
    if let AstNode::VarDecl {
        name,
        value: Some(AstExpr::MethodCall { method, .. }),
        ..
    } = node
    {
        if method.contains("StringBuffer") || method.contains("_StringBuffer") {
            return Some(name.clone());
        }
    }
    if let AstNode::VarDecl {
        name,
        value: Some(AstExpr::ConstructorCall { class_name, .. }),
        ..
    } = node
    {
        if class_name.contains("StringBuffer") {
            return Some(name.clone());
        }
    }
    None
}

fn extract_write_call(node: &AstNode, sb_var: &str) -> Option<StringPart> {
    if let AstNode::ExprStatement(AstExpr::MethodCall {
        receiver: Some(recv),
        method,
        args,
        ..
    }) = node
    {
        if let AstExpr::Variable(name) = recv.as_ref() {
            if name == sb_var && (method == "write" || method == "writeAll") {
                if let Some(arg) = args.first() {
                    return match arg {
                        AstExpr::Literal(DartLiteral::String(s)) => {
                            Some(StringPart::Literal(s.clone()))
                        }
                        // toString() call on a variable -> interpolation
                        AstExpr::MethodCall {
                            receiver: Some(inner_recv),
                            method: inner_method,
                            ..
                        } if inner_method == "toString" => {
                            Some(StringPart::Interpolation(inner_recv.as_ref().clone()))
                        }
                        // Direct variable reference -> interpolation
                        other => Some(StringPart::Interpolation(other.clone())),
                    };
                }
            }
        }
    }
    None
}

fn extract_to_string(node: &AstNode, sb_var: &str) -> Option<String> {
    if let AstNode::VarDecl {
        name,
        value: Some(AstExpr::MethodCall {
            receiver: Some(recv),
            method,
            ..
        }),
        ..
    } = node
    {
        if let AstExpr::Variable(var_name) = recv.as_ref() {
            if var_name == sb_var && method == "toString" {
                return Some(name.clone());
            }
        }
    }
    None
}
