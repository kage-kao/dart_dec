use crate::Architecture;
use serde::{Deserialize, Serialize};

/// Information about a single section in the binary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionInfo {
    pub name: String,
    pub virtual_addr: u64,
    pub file_offset: u64,
    pub size: u64,
    pub is_executable: bool,
    pub is_writable: bool,
}

/// Parsed binary with extracted sections and architecture info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedBinary {
    pub arch: Architecture,
    pub sections: Vec<SectionInfo>,
    pub entry_point: u64,
    /// Cached index into sections for .rodata
    pub rodata_idx: Option<usize>,
    /// Cached index into sections for .text
    pub text_idx: Option<usize>,
    /// Cached index into sections for vm snapshot data
    pub vm_snapshot_data_idx: Option<usize>,
    /// Cached index into sections for vm snapshot instructions
    pub vm_snapshot_instructions_idx: Option<usize>,
    /// Cached index into sections for isolate snapshot data
    pub isolate_snapshot_data_idx: Option<usize>,
    /// Cached index into sections for isolate snapshot instructions
    pub isolate_snapshot_instructions_idx: Option<usize>,
}

impl ParsedBinary {
    /// Get a section by name
    pub fn section_by_name(&self, name: &str) -> Option<&SectionInfo> {
        self.sections.iter().find(|s| s.name == name)
    }

    /// Get the .rodata section
    pub fn rodata(&self) -> Option<&SectionInfo> {
        self.rodata_idx.map(|i| &self.sections[i])
    }

    /// Get the .text section
    pub fn text(&self) -> Option<&SectionInfo> {
        self.text_idx.map(|i| &self.sections[i])
    }

    /// Get the _kDartVmSnapshotData section
    pub fn vm_snapshot_data(&self) -> Option<&SectionInfo> {
        self.vm_snapshot_data_idx.map(|i| &self.sections[i])
    }

    /// Get the _kDartVmSnapshotInstructions section
    pub fn vm_snapshot_instructions(&self) -> Option<&SectionInfo> {
        self.vm_snapshot_instructions_idx.map(|i| &self.sections[i])
    }

    /// Get the _kDartIsolateSnapshotData section
    pub fn isolate_snapshot_data(&self) -> Option<&SectionInfo> {
        self.isolate_snapshot_data_idx.map(|i| &self.sections[i])
    }

    /// Get the _kDartIsolateSnapshotInstructions section
    pub fn isolate_snapshot_instructions(&self) -> Option<&SectionInfo> {
        self.isolate_snapshot_instructions_idx
            .map(|i| &self.sections[i])
    }

    /// Build a new ParsedBinary from sections, auto-detecting special sections
    pub fn new(arch: Architecture, sections: Vec<SectionInfo>, entry_point: u64) -> Self {
        let rodata_idx = sections.iter().position(|s| s.name == ".rodata");
        let text_idx = sections.iter().position(|s| s.name == ".text");
        let vm_snapshot_data_idx = sections
            .iter()
            .position(|s| s.name.contains("kDartVmSnapshotData") || s.name == "_kDartVmSnapshotData");
        let vm_snapshot_instructions_idx = sections.iter().position(|s| {
            s.name.contains("kDartVmSnapshotInstructions")
                || s.name == "_kDartVmSnapshotInstructions"
        });
        let isolate_snapshot_data_idx = sections.iter().position(|s| {
            s.name.contains("kDartIsolateSnapshotData")
                || s.name == "_kDartIsolateSnapshotData"
        });
        let isolate_snapshot_instructions_idx = sections.iter().position(|s| {
            s.name.contains("kDartIsolateSnapshotInstructions")
                || s.name == "_kDartIsolateSnapshotInstructions"
        });

        Self {
            arch,
            sections,
            entry_point,
            rodata_idx,
            text_idx,
            vm_snapshot_data_idx,
            vm_snapshot_instructions_idx,
            isolate_snapshot_data_idx,
            isolate_snapshot_instructions_idx,
        }
    }
}
