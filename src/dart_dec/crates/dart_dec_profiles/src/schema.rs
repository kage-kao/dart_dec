use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root profile schema for a Dart VM version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DartProfile {
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    pub version: String,
    pub channel: Option<String>,
    pub vm_hash: Option<String>,
    pub architecture_specific: HashMap<String, ArchConfig>,
    pub snapshot_header: SnapshotHeaderLayout,
    pub class_layout: HashMap<String, ClassLayout>,
    pub object_tags: ObjectTagsConfig,
    pub class_ids: HashMap<String, u16>,
    pub stubs: HashMap<String, String>,
}

/// Architecture-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchConfig {
    pub compressed_pointers: bool,
    pub pointer_size: u32,
    pub object_alignment: u32,
}

/// Snapshot header layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotHeaderLayout {
    pub magic_offset: u32,
    pub magic_value: String,
    pub version_offset: u32,
    pub features_offset: u32,
    pub base_objects_offset: u32,
}

/// Layout of a single Dart VM class (field offsets and sizes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassLayout {
    #[serde(default)]
    pub size: u32,
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

/// Object tag bit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectTagsConfig {
    pub class_id_mask: String,
    pub class_id_shift: u32,
    pub size_tag_mask: String,
    pub size_tag_shift: u32,
    pub canonical_bit: u32,
    pub old_and_not_marked_bit: u32,
}

impl ObjectTagsConfig {
    /// Parse class_id_mask from hex string
    pub fn class_id_mask_value(&self) -> u64 {
        parse_hex_str(&self.class_id_mask)
    }

    /// Parse size_tag_mask from hex string
    pub fn size_tag_mask_value(&self) -> u64 {
        parse_hex_str(&self.size_tag_mask)
    }
}

/// Parse "0xFFFF" style strings into u64
fn parse_hex_str(s: &str) -> u64 {
    let s = s.trim_start_matches("0x").trim_start_matches("0X");
    u64::from_str_radix(s, 16).unwrap_or(0)
}

impl DartProfile {
    /// Get architecture config for a given arch name
    pub fn arch_config(&self, arch: &str) -> Option<&ArchConfig> {
        self.architecture_specific.get(arch)
    }

    /// Get class layout by name (e.g., "RawClass", "RawFunction")
    pub fn class_layout_by_name(&self, name: &str) -> Option<&ClassLayout> {
        self.class_layout.get(name)
    }

    /// Get class ID by name (e.g., "OneByteString" -> 78)
    pub fn class_id(&self, name: &str) -> Option<u16> {
        self.class_ids.get(name).copied()
    }

    /// Get the pointer size for a given architecture
    pub fn pointer_size(&self, arch: &str) -> u32 {
        self.architecture_specific
            .get(arch)
            .map(|c| c.pointer_size)
            .unwrap_or(8)
    }

    /// Check if compressed pointers are used for a given architecture
    pub fn uses_compressed_pointers(&self, arch: &str) -> bool {
        self.architecture_specific
            .get(arch)
            .map(|c| c.compressed_pointers)
            .unwrap_or(false)
    }
}
