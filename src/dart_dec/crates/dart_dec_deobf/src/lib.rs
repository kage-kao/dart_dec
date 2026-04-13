//! dart_dec_deobf — Deobfuscation and symbol recovery.

pub mod control_flow;
pub mod heuristics;
pub mod string_decrypt;
pub mod symbol_recovery;

use dart_dec_snapshot::types::*;
use dart_dec_snapshot::object_pool::ObjectPool;
use dart_dec_graph::AstNode;

/// Apply all deobfuscation passes
pub fn deobfuscate(
    pool: &ObjectPool,
    ast: &mut AstNode,
    config: &DeobfConfig,
) {
    if config.recover_symbols {
        symbol_recovery::recover_symbols(pool, ast);
    }
    if config.decrypt_strings {
        string_decrypt::decrypt_strings(ast);
    }
    if config.deobf_control_flow {
        control_flow::deobfuscate_control_flow(ast);
    }
    if config.apply_heuristics {
        heuristics::apply_heuristic_naming(pool, ast);
    }
}

/// Configuration for deobfuscation passes
pub struct DeobfConfig {
    pub recover_symbols: bool,
    pub decrypt_strings: bool,
    pub deobf_control_flow: bool,
    pub apply_heuristics: bool,
}

impl Default for DeobfConfig {
    fn default() -> Self {
        Self {
            recover_symbols: true,
            decrypt_strings: true,
            deobf_control_flow: true,
            apply_heuristics: true,
        }
    }
}
