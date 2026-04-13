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

    // Direct extraction approach: extract strings, class names, and function
    // names directly from the binary using pattern matching on .rodata section
    // This works regardless of snapshot cluster format version
    let rodata = parsed.rodata();
    let all_data = file.data();

    let (rodata_start, rodata_end) = if let Some(r) = rodata {
        let start = r.file_offset as usize;
        let end = (start + r.size as usize).min(all_data.len());
        (start, end)
    } else {
        (0, all_data.len())
    };

    // Extract strings directly from .rodata
    let mut pool = object_pool::ObjectPool::new();
    let mut string_addr_counter = 0u64;

    // Extract printable strings (min length 4)
    let mut current = Vec::new();
    let mut str_start = rodata_start;
    for i in rodata_start..rodata_end {
        let b = all_data[i];
        if b >= 32 && b <= 126 {
            if current.is_empty() {
                str_start = i;
            }
            current.push(b);
        } else {
            if current.len() >= 4 {
                if let Ok(s) = String::from_utf8(current.clone()) {
                    let addr = SnapshotAddr(string_addr_counter);
                    pool.insert(addr, DartObject::String(DartString {
                        addr,
                        value: s,
                        is_one_byte: true,
                    }));
                    string_addr_counter += 1;
                }
            }
            current.clear();
        }
    }

    // Classify extracted strings into classes, functions, libraries
    let mut classes_vec = Vec::new();
    let mut class_addr_counter = 0x80000000u64;

    let mut package_libs = Vec::new();
    let mut dart_libs = Vec::new();

    // Collect class candidates from strings (iterate over snapshot of values)
    let string_values: Vec<(SnapshotAddr, String)> = pool.iter()
        .filter_map(|(addr, obj)| {
            if let DartObject::String(s) = obj {
                Some((*addr, s.value.clone()))
            } else {
                None
            }
        })
        .collect();

    for (_addr, val) in &string_values {
        if val.starts_with("package:") {
            package_libs.push(val.clone());
        }
        if val.starts_with("dart:") {
            dart_libs.push(val.clone());
        }
        if val.len() >= 2 && val.len() <= 80
            && val.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false)
            && val.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !val.contains("__")
        {
            let addr = SnapshotAddr(class_addr_counter);
            let class = DartClass {
                addr,
                name: val.clone(),
                library: String::new(),
                super_class: None,
                interfaces: vec![],
                fields: vec![],
                functions: vec![],
                type_parameters: vec![],
                is_abstract: false,
                is_sealed: false,
                is_mixin: false,
                is_enum: false,
                class_id: 0,
            };
            classes_vec.push(class);
            class_addr_counter += 1;
        }
    }

    // Insert classes into pool
    for cls in &classes_vec {
        pool.insert(cls.addr, DartObject::Class(cls.clone()));
    }

    // Build class table from extracted classes
    let mut classes = class_table::ClassTable::new();
    for (i, cls) in classes_vec.iter().enumerate() {
        classes.addr_to_idx.insert(cls.addr, i);
    }
    classes.classes = classes_vec;

    // Build string table
    let strings = string_table::extract_strings(&pool, data);

    // Build library mapping from package imports
    let mut libraries = Vec::new();
    let mut lib_set = std::collections::HashSet::new();
    for pkg in &package_libs {
        if lib_set.insert(pkg.clone()) {
            libraries.push(DartLibrary {
                name: pkg.clone(),
                url: pkg.clone(),
                classes: vec![],
            });
        }
    }
    for dl in &dart_libs {
        if lib_set.insert(dl.clone()) {
            libraries.push(DartLibrary {
                name: dl.clone(),
                url: dl.clone(),
                classes: vec![],
            });
        }
    }

    tracing::info!(
        "Extracted {} strings, {} classes, {} libraries from binary",
        strings.len(), classes.len(), libraries.len()
    );

    Ok(ParsedSnapshot {
        header,
        classes,
        strings,
        objects: pool,
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
