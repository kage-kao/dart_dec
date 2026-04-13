use dart_dec_snapshot::object_pool::ObjectPool;
use dart_dec_snapshot::types::*;
use dart_dec_graph::AstNode;

/// Recover original symbol names from obfuscated binaries
pub fn recover_symbols(pool: &ObjectPool, _ast: &mut AstNode) {
    // Strategy 1: Signature matching
    // If a class has methods build, createState, initState -> StatefulWidget
    for (_addr, class) in pool.classes() {
        let _suggested = suggest_class_name(class, pool);
    }
}

fn suggest_class_name(class: &DartClass, pool: &ObjectPool) -> Option<String> {
    let methods: Vec<String> = class.functions.iter()
        .filter_map(|addr| pool.get(addr))
        .filter_map(|obj| if let DartObject::Function(f) = obj { Some(f.name.clone()) } else { None })
        .collect();

    // Flutter widget patterns
    if methods.iter().any(|m| m.contains("build")) && methods.iter().any(|m| m.contains("createState")) {
        return Some("StatefulWidget".to_string());
    }
    if methods.iter().any(|m| m.contains("build")) && !methods.iter().any(|m| m.contains("createState")) {
        return Some("StatelessWidget".to_string());
    }
    if methods.iter().any(|m| m.contains("initState")) && methods.iter().any(|m| m.contains("dispose")) {
        return Some("State".to_string());
    }

    None
}
