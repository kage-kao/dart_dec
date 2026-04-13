//! dart_dec_snapshot — AOT Snapshot parsing and object pool recovery.
//!
//! Parses Dart AOT snapshots, iterates over the object pool, and
//! recovers Dart objects (classes, functions, strings, etc.) into
//! a semantic tree.

pub mod class_table;
mod header;
pub mod object_pool;
pub mod string_table;
mod stubs;
pub mod types;

pub use class_table::ClassTable;
pub use header::SnapshotHeader;
pub use object_pool::ObjectPool;
pub use string_table::StringTable;
pub use stubs::{StubKind, StubRecognizer};
pub use types::*;

#[cfg(test)]
mod tests;

use dart_dec_core::{Architecture, BinaryFile, CoreError};
use dart_dec_core::sections::ParsedBinary;
use dart_dec_profiles::DartProfile;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("Invalid snapshot magic: expected 0xf5f5dcdc, got 0x{0:08x}")]
    InvalidMagic(u32),

    #[error("Snapshot header too small: {0} bytes")]
    HeaderTooSmall(usize),

    #[error("Object at offset 0x{offset:x} has invalid class_id: {class_id}")]
    InvalidClassId { offset: u64, class_id: u16 },

    #[error("Object pool iteration exceeded bounds at offset 0x{0:x}")]
    PoolBoundsExceeded(u64),

    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    #[error("No snapshot data section found in binary")]
    NoSnapshotData,

    #[error("Profile mismatch: {0}")]
    ProfileMismatch(String),
}

/// Complete parsed snapshot with all recovered objects
#[derive(Debug, Clone)]
pub struct ParsedSnapshot {
    pub header: SnapshotHeader,
    pub classes: ClassTable,
    pub strings: StringTable,
    pub objects: ObjectPool,
    pub libraries: Vec<DartLibrary>,
}

/// Represents a Dart library with its classes
#[derive(Debug, Clone, serde::Serialize)]
pub struct DartLibrary {
    pub name: String,
    pub url: String,
    pub classes: Vec<usize>, // Indices into ClassTable
}

/// Parse a complete AOT snapshot from a binary file
pub fn parse_snapshot(
    file: &BinaryFile,
    parsed: &ParsedBinary,
    profile: &DartProfile,
) -> Result<ParsedSnapshot, SnapshotError> {
    let arch_str = match parsed.arch {
        Architecture::Arm64 => "arm64",
        Architecture::Arm32 => "arm32",
        Architecture::X86_64 => "x86_64",
        Architecture::X86 => "x86_64",
    };

    // Find snapshot data section
    let snapshot_data = parsed
        .isolate_snapshot_data()
        .or_else(|| parsed.rodata())
        .ok_or(SnapshotError::NoSnapshotData)?;

    let data = file.slice(
        snapshot_data.file_offset as usize,
        snapshot_data.size as usize,
    )?;

    // Parse header
    let header = header::parse_header(data, profile)?;

    // Parse object pool
    let base_offset = profile.snapshot_header.base_objects_offset as usize;
    let pointer_size = profile.pointer_size(arch_str);
    let compressed = profile.uses_compressed_pointers(arch_str);
    let objects = object_pool::parse_object_pool(
        data,
        base_offset,
        profile,
        pointer_size,
        compressed,
    )?;

    // Build class table
    let classes = class_table::build_class_table(&objects, profile);

    // Extract strings
    let strings = string_table::extract_strings(&objects, data);

    // Build library mapping
    let libraries = build_library_mapping(&classes, &objects);

    Ok(ParsedSnapshot {
        header,
        classes,
        strings,
        objects,
        libraries,
    })
}

fn build_library_mapping(classes: &ClassTable, _objects: &ObjectPool) -> Vec<DartLibrary> {
    let mut lib_map: ahash::AHashMap<String, Vec<usize>> = ahash::AHashMap::new();

    for (idx, class) in classes.classes.iter().enumerate() {
        let lib_name = class.library.clone();
        lib_map.entry(lib_name).or_default().push(idx);
    }

    lib_map
        .into_iter()
        .map(|(name, class_indices)| DartLibrary {
            url: name.clone(),
            name,
            classes: class_indices,
        })
        .collect()
}
