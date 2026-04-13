/// Fuzz testing setup for dart_dec
///
/// To run fuzz tests:
/// ```bash
/// cargo install cargo-fuzz
/// cd crates/dart_dec_core
/// cargo fuzz init
/// cargo fuzz add elf_parser
/// cargo fuzz run elf_parser -- -max_total_time=60
/// ```
///
/// Fuzz targets:
/// 1. elf_parser - Feed random bytes to ELF parser
/// 2. snapshot_parser - Feed random bytes to snapshot parser
/// 3. ir_lifter - Feed random instructions to IR lifter
///
/// These ensure the decompiler doesn't crash on malformed input,
/// which is critical when analyzing potentially malicious binaries.

// Fuzz target for ELF parser
#[cfg(feature = "fuzz")]
pub fn fuzz_elf_parser(data: &[u8]) {
    use std::io::Write;
    let path = std::path::PathBuf::from("/tmp/fuzz_elf.bin");
    if let Ok(mut f) = std::fs::File::create(&path) {
        let _ = f.write_all(data);
        let _ = dart_dec_core::BinaryFile::open(&path);
    }
    let _ = std::fs::remove_file(&path);
}

// Fuzz target for snapshot parser
#[cfg(feature = "fuzz")]
pub fn fuzz_snapshot_parser(data: &[u8]) {
    let resolver = dart_dec_profiles::ProfileResolver::new();
    if let Some(profile) = resolver.resolve(3, 2, 0) {
        let _ = dart_dec_snapshot::header::parse_header(data, profile);
    }
}

// Fuzz target for disassembler (ARM64)
#[cfg(feature = "fuzz")]
pub fn fuzz_arm64_disasm(data: &[u8]) {
    if let Ok(disasm) = dart_dec_disasm::Arm64Disassembler::new() {
        use dart_dec_disasm::Disassembler;
        let _ = disasm.disassemble(data, 0x1000);
    }
}

// Fuzz target for security scanner
#[cfg(feature = "fuzz")]
pub fn fuzz_secret_scanner(data: &[u8]) {
    if let Ok(text) = std::str::from_utf8(data) {
        let strings = vec![text.to_string()];
        let _ = dart_dec_scan::secrets::scan_secrets(&strings);
    }
}
