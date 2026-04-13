use crate::errors::CoreError;
use crate::memory::BinaryFile;
use crate::sections::{ParsedBinary, SectionInfo};
use crate::Architecture;
use tracing::debug;

/// Parse a PE binary (Windows desktop Flutter) and extract sections
pub fn parse_pe(file: &BinaryFile) -> Result<ParsedBinary, CoreError> {
    let data = file.data();
    let pe = goblin::pe::PE::parse(data).map_err(|e| CoreError::InvalidPe(e.to_string()))?;

    let arch = if pe.is_64 {
        Architecture::X86_64
    } else {
        Architecture::X86
    };

    debug!("PE architecture: {:?}", arch);

    let mut sections = Vec::new();

    for section in &pe.sections {
        let name = String::from_utf8_lossy(
            &section.name[..section
                .name
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(section.name.len())],
        )
        .to_string();

        let characteristics = section.characteristics;
        let is_executable = characteristics & 0x20000000 != 0; // IMAGE_SCN_MEM_EXECUTE
        let is_writable = characteristics & 0x80000000 != 0; // IMAGE_SCN_MEM_WRITE

        sections.push(SectionInfo {
            name: name.clone(),
            virtual_addr: section.virtual_address as u64
                + pe.image_base as u64,
            file_offset: section.pointer_to_raw_data as u64,
            size: section.virtual_size as u64,
            is_executable,
            is_writable,
        });

        debug!(
            "PE section: {} addr=0x{:x} size=0x{:x}",
            name, section.virtual_address, section.virtual_size
        );
    }

    let entry = pe.entry as u64 + pe.image_base as u64;
    Ok(ParsedBinary::new(arch, sections, entry))
}
