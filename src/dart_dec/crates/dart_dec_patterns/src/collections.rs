use dart_dec_graph::*;

/// Recover collection literal patterns (List, Map, Set).
///
/// Dart AOT compiles collection literals into sequences of:
/// 1. AllocateArray + StoreIndexed series -> [element, ...]
/// 2. AllocateArray + paired StoreIndexed -> {key: value, ...}
/// 3. GrowableObjectArray + add() series -> [...] growable
/// 4. LinkedHashSet + add() series -> {...} set literal
pub fn recover_collection_literals(ast: &mut AstNode) {
    walk_collections(ast);
}

fn walk_collections(ast: &mut AstNode) {
    match ast {
        AstNode::Block(stmts) => {
            for stmt in stmts.iter_mut() {
                walk_collections(stmt);
            }

            // Pass 1: Detect List literal pattern
            // var arr = AllocateArray(N); arr[0] = x; arr[1] = y; => var arr = [x, y];
            let mut i = 0;
            while i < stmts.len() {
                if let Some((var_name, count)) = detect_array_alloc(&stmts[i]) {
                    let mut elements = Vec::new();
                    let mut j = i + 1;
                    let mut stores_found = 0;

                    while j < stmts.len() && stores_found < count {
                        if let Some((_idx, value)) =
                            extract_store_indexed(&stmts[j], &var_name)
                        {
                            elements.push(value);
                            stores_found += 1;
                            j += 1;
                        } else {
                            break;
                        }
                    }

                    if stores_found > 0 {
                        let list_lit = AstNode::VarDecl {
                            name: var_name.clone(),
                            dart_type: Some("List".to_string()),
                            value: Some(AstExpr::ListLiteral(elements)),
                            is_final: true,
                            is_late: false,
                        };

                        // Remove the store statements
                        for _ in 0..stores_found {
                            if i + 1 < stmts.len() {
                                stmts.remove(i + 1);
                            }
                        }
                        stmts[i] = list_lit;
                    }
                }

                // Detect Map literal pattern
                // var map = AllocateArray(N*2); map[0]=k0; map[1]=v0; map[2]=k1; map[3]=v1;
                // => var map = {k0: v0, k1: v1};
                if let Some((var_name, count)) = detect_map_alloc(&stmts[i]) {
                    let mut entries = Vec::new();
                    let mut j = i + 1;
                    let mut stores_found = 0;

                    while j + 1 < stmts.len() && stores_found < count {
                        if let (Some((_, key)), Some((_, value))) = (
                            extract_store_indexed(&stmts[j], &var_name),
                            extract_store_indexed(&stmts[j + 1], &var_name),
                        ) {
                            entries.push((key, value));
                            stores_found += 1;
                            j += 2;
                        } else {
                            break;
                        }
                    }

                    if stores_found > 0 {
                        let map_lit = AstNode::VarDecl {
                            name: var_name.clone(),
                            dart_type: Some("Map".to_string()),
                            value: Some(AstExpr::MapLiteral(entries)),
                            is_final: true,
                            is_late: false,
                        };

                        let remove_count = stores_found * 2;
                        for _ in 0..remove_count {
                            if i + 1 < stmts.len() {
                                stmts.remove(i + 1);
                            }
                        }
                        stmts[i] = map_lit;
                    }
                }

                // Detect growable list pattern
                // var list = GrowableObjectArray(); list.add(x); list.add(y); => var list = [x, y];
                if let Some(var_name) = detect_growable_alloc(&stmts[i]) {
                    let mut elements = Vec::new();
                    let mut j = i + 1;

                    while j < stmts.len() {
                        if let Some(value) = extract_add_call(&stmts[j], &var_name) {
                            elements.push(value);
                            j += 1;
                        } else {
                            break;
                        }
                    }

                    if !elements.is_empty() {
                        let list_lit = AstNode::VarDecl {
                            name: var_name.clone(),
                            dart_type: Some("List".to_string()),
                            value: Some(AstExpr::ListLiteral(elements.clone())),
                            is_final: false,
                            is_late: false,
                        };

                        let remove_count = elements.len();
                        for _ in 0..remove_count {
                            if i + 1 < stmts.len() {
                                stmts.remove(i + 1);
                            }
                        }
                        stmts[i] = list_lit;
                    }
                }

                i += 1;
            }
        }
        AstNode::If { then_body, else_body, .. } => {
            walk_collections(then_body);
            if let Some(e) = else_body {
                walk_collections(e);
            }
        }
        AstNode::While { body, .. } | AstNode::DoWhile { body, .. } | AstNode::For { body, .. } => {
            walk_collections(body);
        }
        AstNode::TryCatch { try_body, catches, finally_body } => {
            walk_collections(try_body);
            for c in catches { walk_collections(&mut c.body); }
            if let Some(f) = finally_body { walk_collections(f); }
        }
        _ => {}
    }
}

fn detect_array_alloc(node: &AstNode) -> Option<(String, usize)> {
    if let AstNode::VarDecl {
        name,
        value: Some(AstExpr::MethodCall { method, args, .. }),
        ..
    } = node
    {
        if method.contains("AllocateArray") {
            let count = args.first().and_then(|a| {
                if let AstExpr::Literal(DartLiteral::Int(n)) = a {
                    Some(*n as usize)
                } else {
                    None
                }
            }).unwrap_or(0);
            return Some((name.clone(), count));
        }
    }
    None
}

fn detect_map_alloc(node: &AstNode) -> Option<(String, usize)> {
    if let AstNode::VarDecl {
        name,
        value: Some(AstExpr::MethodCall { method, args, .. }),
        ..
    } = node
    {
        if method.contains("LinkedHashMap") || method.contains("_Map") {
            let count = args.first().and_then(|a| {
                if let AstExpr::Literal(DartLiteral::Int(n)) = a {
                    Some(*n as usize)
                } else {
                    None
                }
            }).unwrap_or(0);
            return Some((name.clone(), count / 2));
        }
    }
    None
}

fn detect_growable_alloc(node: &AstNode) -> Option<String> {
    if let AstNode::VarDecl {
        name,
        value: Some(AstExpr::MethodCall { method, .. }),
        ..
    } = node
    {
        if method.contains("GrowableObjectArray") || method.contains("_GrowableList") {
            return Some(name.clone());
        }
    }
    None
}

fn extract_store_indexed(node: &AstNode, target_var: &str) -> Option<(usize, AstExpr)> {
    if let AstNode::ExprStatement(AstExpr::BinaryOp(lhs, BinOp::Assign, rhs)) = node {
        if let AstExpr::IndexAccess(base, idx) = lhs.as_ref() {
            if let AstExpr::Variable(name) = base.as_ref() {
                if name == target_var {
                    if let AstExpr::Literal(DartLiteral::Int(i)) = idx.as_ref() {
                        return Some((*i as usize, rhs.as_ref().clone()));
                    }
                }
            }
        }
        // Also handle field access style: target.f0 = val
        if let AstExpr::FieldAccess(base, field) = lhs.as_ref() {
            if let AstExpr::Variable(name) = base.as_ref() {
                if name == target_var {
                    let idx = field.trim_start_matches('f').parse::<usize>().unwrap_or(0);
                    return Some((idx, rhs.as_ref().clone()));
                }
            }
        }
    }
    None
}

fn extract_add_call(node: &AstNode, target_var: &str) -> Option<AstExpr> {
    if let AstNode::ExprStatement(AstExpr::MethodCall {
        receiver: Some(recv),
        method,
        args,
        ..
    }) = node
    {
        if let AstExpr::Variable(name) = recv.as_ref() {
            if name == target_var && method == "add" && !args.is_empty() {
                return Some(args[0].clone());
            }
        }
    }
    None
}
