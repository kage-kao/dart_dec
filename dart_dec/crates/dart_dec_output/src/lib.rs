//! dart_dec_output — Output formatters: JSON, SQLite, Dart codegen, SARIF, CSV.

pub mod dart_codegen;
pub mod json;
pub mod protobuf;
pub mod sarif;
pub mod sqlite;

use dart_dec_core::DartVersion;

/// Common output metadata
#[derive(Debug, Clone, serde::Serialize)]
pub struct OutputMeta {
    pub tool: String,
    pub version: String,
    pub timestamp: String,
    pub input_file: String,
    pub input_sha256: String,
    pub dart_version: DartVersion,
    pub architecture: String,
    pub analysis_time_ms: u64,
}

/// Statistics about the decompilation
#[derive(Debug, Clone, serde::Serialize)]
pub struct OutputStats {
    pub total_classes: usize,
    pub total_functions: usize,
    pub total_strings: usize,
    pub decompiled_functions: usize,
    pub failed_functions: usize,
    pub coverage_percent: f64,
}

/// Output format enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Sqlite,
    Dart,
    Sarif,
    Protobuf,
    Dot,
    Csv,
    Jsonl,
}

#[cfg(test)]
mod tests;
