use crate::SnapshotError;
use byteorder::{LittleEndian, ByteOrder};
use dart_dec_profiles::DartProfile;
use serde::Serialize;
use tracing::debug;

/// Parsed snapshot header
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotHeader {
    pub magic: u32,
    pub version: u32,
    pub features: String,
    pub base_objects_offset: u32,
    pub num_objects: u32,
}

/// Parse the snapshot header from raw data
pub fn parse_header(data: &[u8], profile: &DartProfile) -> Result<SnapshotHeader, SnapshotError> {
    let layout = &profile.snapshot_header;

    if data.len() < layout.base_objects_offset as usize {
        return Err(SnapshotError::HeaderTooSmall(data.len()));
    }

    let magic_off = layout.magic_offset as usize;
    let magic = LittleEndian::read_u32(&data[magic_off..magic_off + 4]);

    // Parse expected magic value
    let expected_magic_str = layout.magic_value.trim_start_matches("0x");
    let expected_magic = u32::from_str_radix(expected_magic_str, 16).unwrap_or(0xf5f5dcdc);

    if magic != expected_magic {
        return Err(SnapshotError::InvalidMagic(magic));
    }

    let version_off = layout.version_offset as usize;
    let version = if version_off + 4 <= data.len() {
        LittleEndian::read_u32(&data[version_off..version_off + 4])
    } else {
        0
    };

    let features_off = layout.features_offset as usize;
    let features = extract_features_string(data, features_off);

    // Estimate number of objects from data size and average object size
    let base_off = layout.base_objects_offset as usize;
    let remaining = data.len().saturating_sub(base_off);
    let num_objects = (remaining / 32) as u32; // rough estimate

    debug!(
        "Snapshot header: magic=0x{:08x} version={} features={} num_objects~{}",
        magic, version, features, num_objects
    );

    Ok(SnapshotHeader {
        magic,
        version,
        features,
        base_objects_offset: layout.base_objects_offset,
        num_objects,
    })
}

fn extract_features_string(data: &[u8], offset: usize) -> String {
    if offset >= data.len() {
        return String::new();
    }

    // Features are stored as a null-terminated string or length-prefixed
    // Try null-terminated first
    let end = data[offset..]
        .iter()
        .position(|&b| b == 0 || !b.is_ascii())
        .unwrap_or(256.min(data.len() - offset));

    String::from_utf8_lossy(&data[offset..offset + end]).to_string()
}
