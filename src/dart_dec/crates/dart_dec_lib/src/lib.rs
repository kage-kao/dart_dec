//! dart_dec_lib — Library mode with C FFI for integration into
//! other tools (Python, Ghidra, IDA Pro).

mod ffi;

pub use dart_dec_core::{Architecture, BinaryFile, BinaryFormat, parse_binary, detect_version};
pub use dart_dec_snapshot::{parse_snapshot, ParsedSnapshot};
pub use dart_dec_profiles::ProfileResolver;
pub use dart_dec_disasm::create_disassembler;
pub use dart_dec_lifter::{lift_instructions, PoolResolver, StubResolver};
pub use dart_dec_graph::{CFG, AstNode, generate_dart_code};
pub use dart_dec_output::OutputFormat;

use std::path::Path;

/// High-level API: open a binary and get a decompilation context
pub struct DartDecContext {
    pub file: BinaryFile,
    pub parsed: dart_dec_core::sections::ParsedBinary,
    pub version: dart_dec_core::DartVersion,
    pub snapshot: Option<ParsedSnapshot>,
}

impl DartDecContext {
    /// Open a Dart AOT binary
    pub fn open(path: &Path) -> Result<Self, String> {
        let file = BinaryFile::open(path).map_err(|e| e.to_string())?;
        let parsed = parse_binary(&file).map_err(|e| e.to_string())?;
        let version = detect_version(&file, &parsed).map_err(|e| e.to_string())?;

        Ok(Self {
            file,
            parsed,
            version,
            snapshot: None,
        })
    }

    /// Parse the AOT snapshot
    pub fn parse_snapshot(&mut self) -> Result<(), String> {
        let resolver = ProfileResolver::new();
        let profile = resolver
            .resolve(self.version.major, self.version.minor, self.version.patch)
            .ok_or_else(|| format!("No profile for Dart {}", self.version))?
            .clone();

        let snapshot = parse_snapshot(&self.file, &self.parsed, &profile)
            .map_err(|e| e.to_string())?;
        self.snapshot = Some(snapshot);
        Ok(())
    }

    /// Get classes as JSON string
    pub fn get_classes_json(&self) -> Result<String, String> {
        let snapshot = self.snapshot.as_ref().ok_or("Snapshot not parsed")?;
        serde_json::to_string_pretty(&snapshot.classes.classes)
            .map_err(|e| e.to_string())
    }

    /// Get all strings
    pub fn get_strings(&self) -> Result<Vec<String>, String> {
        let snapshot = self.snapshot.as_ref().ok_or("Snapshot not parsed")?;
        Ok(snapshot.strings.strings.iter().map(|s| s.value.clone()).collect())
    }

    /// Decompile a specific function by class and method name
    pub fn decompile_function(&self, _class_name: &str, _func_name: &str) -> Result<String, String> {
        let _snapshot = self.snapshot.as_ref().ok_or("Snapshot not parsed")?;
        // Full decompilation pipeline would go here
        Ok("// Decompilation placeholder".to_string())
    }
}
