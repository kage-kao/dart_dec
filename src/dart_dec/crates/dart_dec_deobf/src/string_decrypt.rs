use dart_dec_graph::*;

/// Decrypt encrypted strings from third-party protectors.
///
/// Common patterns:
/// 1. XOR-based encryption: each char XORed with key
/// 2. Base64 + custom cipher
/// 3. Runtime decrypt function call before string use
///
/// Detection: Look for function calls that take byte arrays and return strings,
/// where the byte arrays contain high-entropy data.
pub fn decrypt_strings(ast: &mut AstNode) {
    walk_decrypt(ast);
}

fn walk_decrypt(ast: &mut AstNode) {
    match ast {
        AstNode::Block(stmts) => {
            for stmt in stmts.iter_mut() {
                walk_decrypt(stmt);
            }

            // Pattern: var s = decrypt([0x41, 0x23, ...]); => var s = "decrypted";
            let mut i = 0;
            while i < stmts.len() {
                if let Some((var_name, encrypted_bytes, key)) = detect_decrypt_pattern(&stmts[i]) {
                    // Try XOR decryption
                    if let Some(decrypted) = try_xor_decrypt(&encrypted_bytes, key) {
                        stmts[i] = AstNode::VarDecl {
                            name: var_name,
                            dart_type: Some("String".to_string()),
                            value: Some(AstExpr::Literal(DartLiteral::String(decrypted))),
                            is_final: true,
                            is_late: false,
                        };
                    }
                }

                // Pattern: var s = String.fromCharCodes([encoded...]); => simplify
                if let Some((var_name, char_codes)) = detect_from_char_codes(&stmts[i]) {
                    let decoded: String = char_codes.iter().filter_map(|&c| {
                        if c >= 0 && c <= 0x10FFFF {
                            char::from_u32(c as u32)
                        } else {
                            None
                        }
                    }).collect();

                    if !decoded.is_empty() {
                        stmts[i] = AstNode::VarDecl {
                            name: var_name,
                            dart_type: Some("String".to_string()),
                            value: Some(AstExpr::Literal(DartLiteral::String(decoded))),
                            is_final: true,
                            is_late: false,
                        };
                    }
                }

                // Pattern: Concatenation of single-char strings => combined string
                // "h" + "e" + "l" + "l" + "o" => "hello"
                if let Some((var_name, combined)) = detect_char_concat(&stmts[i]) {
                    stmts[i] = AstNode::VarDecl {
                        name: var_name,
                        dart_type: Some("String".to_string()),
                        value: Some(AstExpr::Literal(DartLiteral::String(combined))),
                        is_final: true,
                        is_late: false,
                    };
                }

                i += 1;
            }
        }
        AstNode::If { then_body, else_body, .. } => {
            walk_decrypt(then_body);
            if let Some(e) = else_body { walk_decrypt(e); }
        }
        AstNode::While { body, .. } | AstNode::DoWhile { body, .. } | AstNode::For { body, .. } => {
            walk_decrypt(body);
        }
        AstNode::TryCatch { try_body, catches, finally_body } => {
            walk_decrypt(try_body);
            for c in catches { walk_decrypt(&mut c.body); }
            if let Some(f) = finally_body { walk_decrypt(f); }
        }
        _ => {}
    }
}

fn detect_decrypt_pattern(node: &AstNode) -> Option<(String, Vec<u8>, u8)> {
    if let AstNode::VarDecl {
        name,
        value: Some(AstExpr::MethodCall { method, args, .. }),
        ..
    } = node
    {
        if method.contains("decrypt") || method.contains("_d") || method.contains("decode") {
            if let Some(AstExpr::ListLiteral(items)) = args.first() {
                let bytes: Vec<u8> = items.iter().filter_map(|item| {
                    if let AstExpr::Literal(DartLiteral::Int(v)) = item {
                        Some(*v as u8)
                    } else {
                        None
                    }
                }).collect();

                // Try to extract key from second argument
                let key = args.get(1).and_then(|a| {
                    if let AstExpr::Literal(DartLiteral::Int(k)) = a {
                        Some(*k as u8)
                    } else {
                        None
                    }
                }).unwrap_or(0x42); // Common default XOR key

                if !bytes.is_empty() {
                    return Some((name.clone(), bytes, key));
                }
            }
        }
    }
    None
}

fn try_xor_decrypt(bytes: &[u8], key: u8) -> Option<String> {
    let decrypted: Vec<u8> = bytes.iter().map(|b| b ^ key).collect();
    String::from_utf8(decrypted).ok().filter(|s| {
        // Validate: should be mostly printable ASCII
        s.chars().all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
    })
}

fn detect_from_char_codes(node: &AstNode) -> Option<(String, Vec<i64>)> {
    if let AstNode::VarDecl {
        name,
        value: Some(AstExpr::MethodCall { method, args, .. }),
        ..
    } = node
    {
        if method.contains("fromCharCodes") || method.contains("String.fromCharCode") {
            if let Some(AstExpr::ListLiteral(items)) = args.first() {
                let codes: Vec<i64> = items.iter().filter_map(|item| {
                    if let AstExpr::Literal(DartLiteral::Int(v)) = item {
                        Some(*v)
                    } else {
                        None
                    }
                }).collect();
                if !codes.is_empty() {
                    return Some((name.clone(), codes));
                }
            }
        }
    }
    None
}

fn detect_char_concat(node: &AstNode) -> Option<(String, String)> {
    if let AstNode::VarDecl {
        name,
        value: Some(expr),
        ..
    } = node
    {
        let mut combined = String::new();
        if collect_string_concat(expr, &mut combined) && combined.len() > 1 {
            return Some((name.clone(), combined));
        }
    }
    None
}

fn collect_string_concat(expr: &AstExpr, result: &mut String) -> bool {
    match expr {
        AstExpr::BinaryOp(lhs, BinOp::Add, rhs) => {
            collect_string_concat(lhs, result) && collect_string_concat(rhs, result)
        }
        AstExpr::Literal(DartLiteral::String(s)) if s.len() <= 2 => {
            result.push_str(s);
            true
        }
        _ => false,
    }
}
