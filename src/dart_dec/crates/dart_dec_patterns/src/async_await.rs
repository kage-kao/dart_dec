use dart_dec_graph::*;
use crate::PatternContext;

/// Detect and recover async/await state machines
pub fn recover_async_await(ast: &mut AstNode, context: &PatternContext) {
    // Async functions are compiled into state machines:
    // - A switch on a state variable
    // - Each case corresponds to one await point
    // - Transitions between states represent await expressions
    
    match ast {
        AstNode::Block(stmts) => {
            for stmt in stmts.iter_mut() {
                recover_async_await(stmt, context);
            }
            
            // Look for state machine pattern:
            // switch (state) { case 0: ... case 1: ... }
            if let Some(state_machine_idx) = find_state_machine_switch(stmts) {
                if let AstNode::Switch { cases, .. } = &stmts[state_machine_idx] {
                    let mut async_stmts = Vec::new();
                    for (_, body) in cases {
                        // Each case body becomes a sequential statement
                        // with await insertions at state transitions
                        let mut case_stmts = extract_case_statements(body);
                        async_stmts.append(&mut case_stmts);
                    }
                    if !async_stmts.is_empty() {
                        stmts.splice(state_machine_idx..=state_machine_idx, async_stmts);
                    }
                }
            }
        }
        AstNode::If { then_body, else_body, .. } => {
            recover_async_await(then_body, context);
            if let Some(e) = else_body {
                recover_async_await(e, context);
            }
        }
        AstNode::While { body, .. } | AstNode::DoWhile { body, .. } => {
            recover_async_await(body, context);
        }
        _ => {}
    }
}

fn find_state_machine_switch(stmts: &[AstNode]) -> Option<usize> {
    stmts.iter().position(|s| {
        matches!(s, AstNode::Switch { subject, .. } if is_state_variable(subject))
    })
}

fn is_state_variable(expr: &AstExpr) -> bool {
    match expr {
        AstExpr::Variable(name) => name.contains("state") || name.contains("_state"),
        AstExpr::FieldAccess(_, field) => field.contains("state"),
        _ => false,
    }
}

fn extract_case_statements(body: &AstNode) -> Vec<AstNode> {
    match body {
        AstNode::Block(stmts) => {
            let mut result = Vec::new();
            for stmt in stmts {
                // Convert state transitions to await expressions
                match stmt {
                    AstNode::ExprStatement(AstExpr::MethodCall { method, args, .. })
                        if method.contains("async_op") || method.contains("_asyncStar") =>
                    {
                        if let Some(arg) = args.first() {
                            result.push(AstNode::Await(arg.clone()));
                        }
                    }
                    other => result.push(other.clone()),
                }
            }
            result
        }
        other => vec![other.clone()],
    }
}
