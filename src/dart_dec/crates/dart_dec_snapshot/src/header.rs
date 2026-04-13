use crate::SnapshotError;
use byteorder::{LittleEndian, ByteOrder};
use dart_dec_profiles::DartProfile;
use serde::Serialize;
use tracing::{debug, info};

/// Parsed snapshot header
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotHeader {
    pub magic: u32,
    pub version: u32,
    pub features: String,
    pub base_objects_offset: u32,
    pub num_objects: u32,
    pub snapshot_size: u32,
    pub sdk_hash: String,
}

/// Parse the snapshot header from raw data
pub fn parse_header(data: &[u8], _profile: &DartProfile) -> Result<SnapshotHeader, SnapshotError> {
    if data.len() < 64 {
        return Err(SnapshotError::HeaderTooSmall(data.len()));
    }

    // Read magic (first 4 bytes)
    let magic_raw = LittleEndian::read_u32(&data[0..4]);

    // Accept both byte orders of the magic
    let magic = if magic_raw == 0xf5f5dcdc || magic_raw == 0xdcdcf5f5 {
        magic_raw
    } else {
        return Err(SnapshotError::InvalidMagic(magic_raw));
    };

    // Snapshot size (bytes 4-7)
    let snapshot_size = LittleEndian::read_u32(&data[4..8]);

    // Version kind (bytes 12-15)
    let version = LittleEndian::read_u32(&data[12..16]);

    // SDK hash starts at byte 20 — 32 hex chars
    let hash_start = 20;
    let hash_end = (hash_start + 32).min(data.len());
    let sdk_hash = if hash_end <= data.len() {
        let hash_bytes = &data[hash_start..hash_end];
        if hash_bytes.iter().all(|&b| b.is_ascii_hexdigit()) {
            String::from_utf8_lossy(hash_bytes).to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Features string starts right after hash (offset 52)
    let features_start = hash_end;
    let features = extract_null_terminated_string(data, features_start);

    // Base objects offset — after the header
    // Find end of features string + alignment
    let header_end = features_start + features.len() + 1; // +1 for null terminator
    let aligned_offset = (header_end + 7) & !7; // 8-byte align

    let num_objects_estimate = if data.len() > aligned_offset {
        ((data.len() - aligned_offset) / 16) as u32 // rough estimate
    } else {
        0
    };

    info!(
        "Snapshot: magic=0x{:08x} size={} version={} hash={} features={}",
        magic, snapshot_size, version, sdk_hash, &features[..features.len().min(60)]
    );

    Ok(SnapshotHeader {
        magic,
        version,
        features,
        base_objects_offset: aligned_offset as u32,
        num_objects: num_objects_estimate,
        snapshot_size,
        sdk_hash,
    })
}

fn extract_null_terminated_string(data: &[u8], offset: usize) -> String {
    if offset >= data.len() {
        return String::new();
    }

    let mut end = offset;
    while end < data.len() && data[end] != 0 && end - offset < 1024 {
        end += 1;
    }

    String::from_utf8_lossy(&data[offset..end]).to_string()
}
