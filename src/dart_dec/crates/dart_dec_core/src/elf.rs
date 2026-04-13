use crate::errors::CoreError;
use crate::memory::BinaryFile;
use crate::sections::{ParsedBinary, SectionInfo};
use crate::Architecture;
use tracing::{debug, warn};

/// Parse an ELF binary and extract sections
pub fn parse_elf(file: &BinaryFile) -> Result<ParsedBinary, CoreError> {
    let data = file.data();
    let elf = goblin::elf::Elf::parse(data).map_err(|e| CoreError::InvalidElf(e.to_string()))?;

    // Determine architecture from e_machine
    let arch = match elf.header.e_machine {
        goblin::elf::header::EM_AARCH64 => Architecture::Arm64,
        goblin::elf::header::EM_ARM => Architecture::Arm32,
        goblin::elf::header::EM_X86_64 => Architecture::X86_64,
        goblin::elf::header::EM_386 => Architecture::X86,
        other => {
            return Err(CoreError::UnsupportedArchitecture(format!(
                "ELF e_machine: 0x{:x}",
                other
            )))
        }
    };

    debug!("ELF architecture: {:?}", arch);

    // Extract all sections
    let mut sections = Vec::new();
    for sh in &elf.section_headers {
        let name = elf
            .shdr_strtab
            .get_at(sh.sh_name)
            .unwrap_or("")
            .to_string();

        if name.is_empty() {
            continue;
        }

        let is_executable = sh.sh_flags & goblin::elf::section_header::SHF_EXECINSTR as u64 != 0;
        let is_writable = sh.sh_flags & goblin::elf::section_header::SHF_WRITE as u64 != 0;

        sections.push(SectionInfo {
            name: name.clone(),
            virtual_addr: sh.sh_addr,
            file_offset: sh.sh_offset,
            size: sh.sh_size,
            is_executable,
            is_writable,
        });

        debug!(
            "Section: {} addr=0x{:x} offset=0x{:x} size=0x{:x}",
            name, sh.sh_addr, sh.sh_offset, sh.sh_size
        );
    }

    // Also extract from dynamic symbols for Dart-specific sections
    for sym in elf.dynsyms.iter() {
        if let Some(name) = elf.dynstrtab.get_at(sym.st_name) {
            if name.starts_with("_kDart") {
                // Check if we already have this section
                if !sections.iter().any(|s| s.name == name) {
                    debug!("Found Dart symbol: {} at 0x{:x}", name, sym.st_value);
                    sections.push(SectionInfo {
                        name: name.to_string(),
                        virtual_addr: sym.st_value,
                        file_offset: vaddr_to_file_offset(&elf, sym.st_value)
                            .unwrap_or(sym.st_value),
                        size: sym.st_size,
                        is_executable: false,
                        is_writable: false,
                    });
                }
            }
        }
    }

    // Validate critical sections
    let has_text = sections.iter().any(|s| s.name == ".text");
    if !has_text {
        warn!("No .text section found — binary may be stripped or packed");
    }

    let has_rodata = sections.iter().any(|s| s.name == ".rodata");
    if !has_rodata {
        warn!("No .rodata section found");
    }

    // Check for non-standard sections (possible packing/protection)
    for s in &sections {
        if s.name.starts_with(".packed")
            || s.name.starts_with(".encrypt")
            || s.name.starts_with(".protect")
        {
            warn!(
                "Non-standard section '{}' detected — may indicate packing/protection",
                s.name
            );
        }
    }

    let entry = elf.entry;
    Ok(ParsedBinary::new(arch, sections, entry))
}

/// Convert a virtual address to file offset using ELF program headers
fn vaddr_to_file_offset(elf: &goblin::elf::Elf, vaddr: u64) -> Option<u64> {
    for ph in &elf.program_headers {
        if ph.p_type == goblin::elf::program_header::PT_LOAD
            && vaddr >= ph.p_vaddr
            && vaddr < ph.p_vaddr + ph.p_memsz
        {
            return Some(vaddr - ph.p_vaddr + ph.p_offset);
        }
    }
    None
}
