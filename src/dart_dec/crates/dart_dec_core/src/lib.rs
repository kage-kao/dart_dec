//! dart_dec_core — Core binary parsing, section mapping, and version detection.
//!
//! This crate provides zero-copy memory-mapped file access, cross-format
//! binary parsing (ELF, Mach-O, PE), section extraction, architecture
//! detection, and Dart VM version identification.

mod elf;
mod errors;
mod macho;
mod memory;
mod pe;
pub mod sections;
pub mod version;

#[cfg(test)]
mod tests;

pub use elf::parse_elf;
pub use errors::CoreError;
pub use macho::parse_macho;
pub use memory::BinaryFile;
pub use pe::parse_pe;
pub use sections::{ParsedBinary, SectionInfo};
pub use version::{Channel, DartVersion, DetectionMethod};

/// Supported binary formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BinaryFormat {
    Elf,
    MachO,
    Pe,
}

/// Supported CPU architectures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Architecture {
    Arm64,
    Arm32,
    X86_64,
    X86,
}

impl std::fmt::Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Architecture::Arm64 => write!(f, "arm64"),
            Architecture::Arm32 => write!(f, "arm32"),
            Architecture::X86_64 => write!(f, "x86_64"),
            Architecture::X86 => write!(f, "x86"),
        }
    }
}

impl std::fmt::Display for BinaryFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryFormat::Elf => write!(f, "ELF"),
            BinaryFormat::MachO => write!(f, "Mach-O"),
            BinaryFormat::Pe => write!(f, "PE"),
        }
    }
}

/// Parse any supported binary file and return structured information
pub fn parse_binary(file: &BinaryFile) -> Result<ParsedBinary, CoreError> {
    match file.format() {
        BinaryFormat::Elf => parse_elf(file),
        BinaryFormat::MachO => parse_macho(file),
        BinaryFormat::Pe => parse_pe(file),
    }
}

/// Detect Dart VM version from a parsed binary
pub fn detect_version(file: &BinaryFile, parsed: &ParsedBinary) -> Result<DartVersion, CoreError> {
    version::detect_dart_version(file, parsed)
}
