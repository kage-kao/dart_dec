use crate::errors::CoreError;
use crate::memory::BinaryFile;
use crate::sections::{ParsedBinary, SectionInfo};
use crate::Architecture;
use tracing::debug;

/// Parse a Mach-O binary (iOS libapp.dylib) and extract sections
pub fn parse_macho(file: &BinaryFile) -> Result<ParsedBinary, CoreError> {
    let data = file.data();
    let macho =
        goblin::mach::Mach::parse(data).map_err(|e| CoreError::InvalidMachO(e.to_string()))?;

    match macho {
        goblin::mach::Mach::Binary(macho) => parse_single_macho(&macho, data),
        goblin::mach::Mach::Fat(fat) => {
            // For fat binaries, prefer arm64
            for arch in fat.iter_arches().flatten() {
                let offset = arch.offset as usize;
                let size = arch.size as usize;
                if offset + size <= data.len() {
                    let slice = &data[offset..offset + size];
                    if let Ok(macho) = goblin::mach::MachO::parse(slice, 0) {
                        let parsed = parse_single_macho(&macho, slice)?;
                        if parsed.arch == Architecture::Arm64 {
                            return Ok(parsed);
                        }
                    }
                }
            }
            Err(CoreError::InvalidMachO(
                "No suitable architecture in fat binary".to_string(),
            ))
        }
    }
}

fn parse_single_macho(
    macho: &goblin::mach::MachO,
    _data: &[u8],
) -> Result<ParsedBinary, CoreError> {
    use goblin::mach::cputype::*;

    let arch = match macho.header.cputype() {
        CPU_TYPE_ARM64 => Architecture::Arm64,
        CPU_TYPE_ARM => Architecture::Arm32,
        CPU_TYPE_X86_64 => Architecture::X86_64,
        CPU_TYPE_X86 => Architecture::X86,
        other => {
            return Err(CoreError::UnsupportedArchitecture(format!(
                "Mach-O cputype: 0x{:x}",
                other
            )))
        }
    };

    debug!("Mach-O architecture: {:?}", arch);

    let mut sections = Vec::new();

    for segment in &macho.segments {
        let seg_name = segment.name().unwrap_or("").to_string();
        for (section, _) in segment.sections().unwrap_or_default() {
            let sect_name = section.name().unwrap_or("").to_string();
            let full_name = if seg_name.is_empty() {
                sect_name.clone()
            } else {
                format!("{}.{}", seg_name, sect_name)
            };

            sections.push(SectionInfo {
                name: full_name,
                virtual_addr: section.addr,
                file_offset: section.offset as u64,
                size: section.size,
                is_executable: segment.flags & 0x4 != 0,
                is_writable: segment.flags & 0x2 != 0,
            });

            debug!(
                "Mach-O section: {} addr=0x{:x} size=0x{:x}",
                sect_name, section.addr, section.size
            );
        }
    }

    let entry = macho.entry as u64;
    Ok(ParsedBinary::new(arch, sections, entry))
}
