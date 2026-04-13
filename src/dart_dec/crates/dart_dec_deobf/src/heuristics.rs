use dart_dec_snapshot::object_pool::ObjectPool;
use dart_dec_snapshot::types::*;
use dart_dec_graph::*;

/// Apply heuristic naming to obfuscated symbols
pub fn apply_heuristic_naming(pool: &ObjectPool, _ast: &mut AstNode) {
    for (_addr, func) in pool.functions() {
        let _name = suggest_function_name(func, pool);
    }
}

fn suggest_function_name(func: &DartFunction, _pool: &ObjectPool) -> Option<String> {
    // HTTP client pattern
    if func.name.contains("http") || func.name.contains("Http") {
        return Some(format!("fetch_{}", func.name));
    }
    // Widget build pattern
    if func.kind == FunctionKind::RegularFunction && func.return_type.as_deref() == Some("Widget") {
        return Some("build".to_string());
    }
    None
}
