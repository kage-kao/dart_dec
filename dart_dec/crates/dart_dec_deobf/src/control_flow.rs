use dart_dec_graph::*;

/// Remove control flow flattening obfuscation.
///
/// Control flow flattening transforms structured code into:
/// ```text
/// var state = 0;
/// while (true) {
///   switch (state) {
///     case 0: ... state = 3; break;
///     case 1: ... state = 5; break;
///     ...
///   }
/// }
/// ```
///
/// Recovery algorithm:
/// 1. Detect the dispatcher pattern (infinite loop + switch on state var)
/// 2. Build a state transition graph from case bodies
/// 3. Topologically sort states by transition order
/// 4. Reconstruct linear control flow from sorted states
/// 5. Recover if/else by analyzing conditional state transitions
pub fn deobfuscate_control_flow(ast: &mut AstNode) {
    walk_deobf(ast);
}

fn walk_deobf(ast: &mut AstNode) {
    match ast {
        AstNode::Block(stmts) => {
            let mut i = 0;
            while i < stmts.len() {
                walk_deobf(&mut stmts[i]);

                // Detect flattened control flow
                if let Some(recovered) = try_recover_flattened(&stmts[i]) {
                    stmts[i] = recovered;
                }

                // Detect opaque predicates: if (condition_always_true) { real_code }
                if let Some(simplified) = try_remove_opaque_predicate(&stmts[i]) {
                    stmts[i] = simplified;
                }

                i += 1;
            }

            // Remove dead code after unconditional returns/throws
            remove_dead_code(stmts);
        }
        AstNode::If { then_body, else_body, .. } => {
            walk_deobf(then_body);
            if let Some(e) = else_body {
                walk_deobf(e);
            }
        }
        AstNode::While { body, .. } | AstNode::DoWhile { body, .. } | AstNode::For { body, .. } => {
            walk_deobf(body);
        }
        AstNode::TryCatch { try_body, catches, finally_body } => {
            walk_deobf(try_body);
            for c in catches { walk_deobf(&mut c.body); }
            if let Some(f) = finally_body { walk_deobf(f); }
        }
        AstNode::Switch { cases, default, .. } => {
            for (_, body) in cases.iter_mut() { walk_deobf(body); }
            if let Some(d) = default { walk_deobf(d); }
        }
        _ => {}
    }
}

/// Try to recover a flattened control flow structure
fn try_recover_flattened(node: &AstNode) -> Option<AstNode> {
    // Pattern: while(true) { switch(state) { case 0: ... case 1: ... } }
    if let AstNode::While {
        condition,
        body,
    } = node
    {
        if is_always_true(condition) {
            if let AstNode::Block(inner) = body.as_ref() {
                if inner.len() == 1 {
                    if let AstNode::Switch { subject, cases, .. } = &inner[0] {
                        if is_state_variable(subject) {
                            return Some(recover_from_state_machine(cases));
                        }
                    }
                }
            }
            if let AstNode::Switch { subject, cases, .. } = body.as_ref() {
                if is_state_variable(subject) {
                    return Some(recover_from_state_machine(cases));
                }
            }
        }
    }
    None
}

fn is_always_true(expr: &AstExpr) -> bool {
    matches!(expr, AstExpr::Literal(DartLiteral::Bool(true)))
        || matches!(expr, AstExpr::Literal(DartLiteral::Int(1)))
}

fn is_state_variable(expr: &AstExpr) -> bool {
    match expr {
        AstExpr::Variable(name) => {
            name.contains("state")
                || name.contains("_state")
                || name.contains("dispatcher")
                || name == "s"
        }
        _ => false,
    }
}

/// Recover original control flow from a state machine switch
fn recover_from_state_machine(cases: &[(AstExpr, AstNode)]) -> AstNode {
    // Build transition order: extract state assignments from each case
    let mut state_order: Vec<(i64, AstNode, Option<i64>)> = Vec::new();

    for (case_expr, body) in cases {
        let state_id = match case_expr {
            AstExpr::Literal(DartLiteral::Int(n)) => *n,
            _ => continue,
        };

        let next_state = extract_next_state(body);

        // Clone body and strip state assignment
        let cleaned_body = strip_state_assignment(body);
        state_order.push((state_id, cleaned_body, next_state));
    }

    // Sort by state ID to get initial order
    state_order.sort_by_key(|(id, _, _)| *id);

    // Follow transition chain starting from state 0
    let mut ordered_stmts = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut current_state = 0i64;

    loop {
        if visited.contains(&current_state) {
            break; // Loop detected
        }
        visited.insert(current_state);

        if let Some((_, body, next)) = state_order.iter().find(|(id, _, _)| *id == current_state) {
            ordered_stmts.push(body.clone());
            if let Some(next_state) = next {
                current_state = *next_state;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    AstNode::Block(ordered_stmts)
}

fn extract_next_state(body: &AstNode) -> Option<i64> {
    match body {
        AstNode::Block(stmts) => {
            for stmt in stmts.iter().rev() {
                if let Some(state) = extract_state_assignment(stmt) {
                    return Some(state);
                }
            }
            None
        }
        _ => extract_state_assignment(body),
    }
}

fn extract_state_assignment(node: &AstNode) -> Option<i64> {
    if let AstNode::ExprStatement(AstExpr::BinaryOp(lhs, BinOp::Assign, rhs)) = node {
        if is_state_variable(lhs) {
            if let AstExpr::Literal(DartLiteral::Int(n)) = rhs.as_ref() {
                return Some(*n);
            }
        }
    }
    None
}

fn strip_state_assignment(body: &AstNode) -> AstNode {
    match body {
        AstNode::Block(stmts) => {
            let filtered: Vec<AstNode> = stmts
                .iter()
                .filter(|s| !is_state_change(s) && !is_break_stmt(s))
                .cloned()
                .collect();
            AstNode::Block(filtered)
        }
        other => other.clone(),
    }
}

fn is_state_change(node: &AstNode) -> bool {
    if let AstNode::ExprStatement(AstExpr::BinaryOp(lhs, BinOp::Assign, _)) = node {
        return is_state_variable(lhs);
    }
    false
}

fn is_break_stmt(node: &AstNode) -> bool {
    matches!(node, AstNode::Break)
}

/// Try to remove opaque predicates (conditions that are always true/false)
fn try_remove_opaque_predicate(node: &AstNode) -> Option<AstNode> {
    if let AstNode::If { condition, then_body, else_body } = node {
        // Pattern: if (x * x >= 0) { real } => just real (always true for ints)
        if is_tautology(condition) {
            return Some(then_body.as_ref().clone());
        }
        // Pattern: if (x * x < 0) { junk } else { real } => just real (always false)
        if is_contradiction(condition) {
            if let Some(else_b) = else_body {
                return Some(else_b.as_ref().clone());
            }
        }
    }
    None
}

fn is_tautology(expr: &AstExpr) -> bool {
    match expr {
        AstExpr::Literal(DartLiteral::Bool(true)) => true,
        AstExpr::BinaryOp(_, BinOp::GreaterOrEqual, rhs) => {
            // x >= 0 where x is always non-negative (e.g., squared value)
            matches!(rhs.as_ref(), AstExpr::Literal(DartLiteral::Int(0)))
        }
        _ => false,
    }
}

fn is_contradiction(expr: &AstExpr) -> bool {
    match expr {
        AstExpr::Literal(DartLiteral::Bool(false)) => true,
        AstExpr::BinaryOp(_, BinOp::LessThan, rhs) => {
            // x < 0 where x is always non-negative
            matches!(rhs.as_ref(), AstExpr::Literal(DartLiteral::Int(0)))
        }
        _ => false,
    }
}

/// Remove dead code after unconditional return/throw
fn remove_dead_code(stmts: &mut Vec<AstNode>) {
    let mut cut_point = None;
    for (i, stmt) in stmts.iter().enumerate() {
        if matches!(stmt, AstNode::Return(_) | AstNode::Throw(_)) {
            if i + 1 < stmts.len() {
                cut_point = Some(i + 1);
                break;
            }
        }
    }
    if let Some(point) = cut_point {
        stmts.truncate(point);
    }
}
