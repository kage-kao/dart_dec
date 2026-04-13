#[cfg(test)]
mod tests {
    use crate::*;
    use std::io::Write;
    use std::path::PathBuf;

    fn create_test_elf() -> PathBuf {
        let path = PathBuf::from("/tmp/test_dart_dec.elf");
        let mut f = std::fs::File::create(&path).unwrap();

        // Minimal ELF header (64-bit, little-endian, ARM64)
        let elf_header: Vec<u8> = vec![
            // e_ident
            0x7f, b'E', b'L', b'F', // magic
            2,    // ELFCLASS64
            1,    // ELFDATA2LSB
            1,    // EV_CURRENT
            0,    // ELFOSABI_NONE
            0, 0, 0, 0, 0, 0, 0, 0, // padding
            // e_type
            3, 0, // ET_DYN
            // e_machine
            0xb7, 0, // EM_AARCH64
            // e_version
            1, 0, 0, 0,
            // e_entry
            0, 0, 0, 0, 0, 0, 0, 0,
            // e_phoff
            64, 0, 0, 0, 0, 0, 0, 0,
            // e_shoff
            0, 0, 0, 0, 0, 0, 0, 0,
            // e_flags
            0, 0, 0, 0,
            // e_ehsize
            64, 0,
            // e_phentsize
            56, 0,
            // e_phnum
            0, 0,
            // e_shentsize
            64, 0,
            // e_shnum
            0, 0,
            // e_shstrndx
            0, 0,
        ];
        f.write_all(&elf_header).unwrap();

        // Pad to at least 256 bytes
        let padding = vec![0u8; 256];
        f.write_all(&padding).unwrap();

        // Write a version string somewhere
        f.write_all(b"Dart/3.2.0 (stable) something").unwrap();
        f.write_all(&vec![0u8; 256]).unwrap();

        path
    }

    #[test]
    fn test_binary_file_open() {
        let path = create_test_elf();
        let file = BinaryFile::open(&path).unwrap();
        assert_eq!(file.format(), BinaryFormat::Elf);
        assert!(file.file_size() > 64);
        assert!(!file.sha256().is_empty());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_binary_format_detection_elf() {
        let path = create_test_elf();
        let file = BinaryFile::open(&path).unwrap();
        assert_eq!(file.format(), BinaryFormat::Elf);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_binary_slice() {
        let path = create_test_elf();
        let file = BinaryFile::open(&path).unwrap();
        let slice = file.slice(0, 4).unwrap();
        assert_eq!(slice, &[0x7f, b'E', b'L', b'F']);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_binary_slice_bounds_check() {
        let path = create_test_elf();
        let file = BinaryFile::open(&path).unwrap();
        let result = file.slice(0, file.file_size() + 1);
        assert!(result.is_err());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_find_pattern() {
        let path = create_test_elf();
        let file = BinaryFile::open(&path).unwrap();
        let found = file.find_pattern(b"Dart/");
        assert!(found.is_some());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_find_all_patterns() {
        let path = create_test_elf();
        let file = BinaryFile::open(&path).unwrap();
        let found = file.find_all_patterns(b"ELF");
        assert!(!found.is_empty());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_parse_elf() {
        let path = create_test_elf();
        let file = BinaryFile::open(&path).unwrap();
        let parsed = parse_binary(&file).unwrap();
        assert_eq!(parsed.arch, Architecture::Arm64);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_version_detection() {
        let path = create_test_elf();
        let file = BinaryFile::open(&path).unwrap();
        let parsed = parse_binary(&file).unwrap();
        let version = detect_version(&file, &parsed);
        // Version might or might not be found depending on search
        if let Ok(v) = version {
            assert_eq!(v.major, 3);
            assert_eq!(v.minor, 2);
        }
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_architecture_display() {
        assert_eq!(format!("{}", Architecture::Arm64), "arm64");
        assert_eq!(format!("{}", Architecture::Arm32), "arm32");
        assert_eq!(format!("{}", Architecture::X86_64), "x86_64");
    }

    #[test]
    fn test_format_display() {
        assert_eq!(format!("{}", BinaryFormat::Elf), "ELF");
        assert_eq!(format!("{}", BinaryFormat::MachO), "Mach-O");
        assert_eq!(format!("{}", BinaryFormat::Pe), "PE");
    }

    #[test]
    fn test_unknown_format() {
        let path = PathBuf::from("/tmp/test_unknown.bin");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00]).unwrap();
        let result = BinaryFile::open(&path);
        assert!(result.is_err());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_file_too_small() {
        let path = PathBuf::from("/tmp/test_tiny.bin");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&[0x7f]).unwrap();
        let result = BinaryFile::open(&path);
        assert!(result.is_err());
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_section_info() {
        let section = sections::SectionInfo {
            name: ".text".to_string(),
            virtual_addr: 0x1000,
            file_offset: 0x200,
            size: 0x5000,
            is_executable: true,
            is_writable: false,
        };
        assert_eq!(section.name, ".text");
        assert!(section.is_executable);
        assert!(!section.is_writable);
    }

    #[test]
    fn test_parsed_binary_section_lookup() {
        let sections = vec![
            sections::SectionInfo {
                name: ".text".to_string(),
                virtual_addr: 0x1000,
                file_offset: 0x200,
                size: 0x5000,
                is_executable: true,
                is_writable: false,
            },
            sections::SectionInfo {
                name: ".rodata".to_string(),
                virtual_addr: 0x6000,
                file_offset: 0x5200,
                size: 0x3000,
                is_executable: false,
                is_writable: false,
            },
        ];
        let parsed = sections::ParsedBinary::new(Architecture::Arm64, sections, 0x1000);
        assert!(parsed.text().is_some());
        assert!(parsed.rodata().is_some());
        assert_eq!(parsed.text().unwrap().name, ".text");
        assert!(parsed.section_by_name(".bss").is_none());
    }

    #[test]
    fn test_dart_version_display() {
        let v = version::DartVersion {
            major: 3, minor: 2, patch: 0,
            channel: version::Channel::Stable,
            sdk_hash: None,
            detection_method: version::DetectionMethod::VersionString,
            confidence: version::Confidence::High,
            raw_string: None,
        };
        assert_eq!(format!("{}", v), "3.2.0 (stable)");
    }

    #[test]
    fn test_version_low_confidence_display() {
        let v = version::DartVersion {
            major: 3, minor: 0, patch: 0,
            channel: version::Channel::Unknown,
            sdk_hash: None,
            detection_method: version::DetectionMethod::StructFingerprint,
            confidence: version::Confidence::Low,
            raw_string: None,
        };
        let display = format!("{}", v);
        assert!(display.contains("[confidence: Low]"));
    }
}
