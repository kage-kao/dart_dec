//! dart_dec_patterns — High-level Dart pattern recovery.

pub mod async_await;
pub mod cascade;
pub mod closures;
pub mod collections;
pub mod extensions;
pub mod null_safety;
pub mod records;
pub mod sealed;
pub mod streams;
pub mod string_interpolation;

use dart_dec_graph::*;

/// Apply all pattern recovery passes to an AST
pub fn recover_patterns(ast: &mut AstNode, context: &PatternContext) {
    async_await::recover_async_await(ast, context);
    null_safety::recover_null_safety(ast);
    collections::recover_collection_literals(ast);
    closures::recover_closures(ast);
    records::recover_records(ast, context);
    sealed::recover_sealed_patterns(ast, context);
    streams::recover_streams(ast, context);
    string_interpolation::recover_string_interpolation(ast);
    cascade::recover_cascade(ast);
}

/// Context for pattern recovery (class info, function metadata, etc.)
pub struct PatternContext {
    pub class_names: ahash::AHashMap<u64, String>,
    pub function_names: ahash::AHashMap<u64, String>,
    pub async_functions: Vec<u64>,
    pub generator_functions: Vec<u64>,
    pub sealed_classes: Vec<String>,
}

impl PatternContext {
    pub fn new() -> Self {
        Self {
            class_names: ahash::AHashMap::new(),
            function_names: ahash::AHashMap::new(),
            async_functions: vec![],
            generator_functions: vec![],
            sealed_classes: vec![],
        }
    }
}

impl Default for PatternContext {
    fn default() -> Self {
        Self::new()
    }
}
