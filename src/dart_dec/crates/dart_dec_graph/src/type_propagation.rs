use crate::cfg::CFG;
use dart_dec_lifter::IR;
use ahash::AHashMap;

/// Propagate type information through the CFG
pub fn propagate_types(cfg: &CFG) -> AHashMap<u16, String> {
    let mut type_map: AHashMap<u16, String> = AHashMap::new();

    // Forward pass: collect type information from known sources
    for block in cfg.blocks() {
        for ir in &block.instructions {
            match ir {
                IR::Call { dst: Some(reg), kind, .. } => {
                    match kind {
                        dart_dec_lifter::CallKind::Stub(dart_dec_lifter::StubCallKind::AllocateObject) => {
                            // AllocateObject returns a specific class
                            type_map.insert(reg.0, "Object".to_string());
                        }
                        dart_dec_lifter::CallKind::Stub(dart_dec_lifter::StubCallKind::AllocateArray) => {
                            type_map.insert(reg.0, "List".to_string());
                        }
                        _ => {}
                    }
                }
                IR::LoadPool { dst, resolved: Some(name), .. } => {
                    if name.starts_with("\"") {
                        type_map.insert(dst.0, "String".to_string());
                    } else if name.starts_with("int:") {
                        type_map.insert(dst.0, "int".to_string());
                    } else if name.starts_with("double:") {
                        type_map.insert(dst.0, "double".to_string());
                    } else if name.starts_with("bool:") {
                        type_map.insert(dst.0, "bool".to_string());
                    }
                }
                IR::Assign(dst, dart_dec_lifter::Operand::Imm(_)) => {
                    type_map.insert(dst.0, "int".to_string());
                }
                IR::TypeCheck { src, target_type, is_cast } => {
                    if *is_cast {
                        type_map.insert(src.0, target_type.name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    type_map
}
