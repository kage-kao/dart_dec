/// YARA scanning support (requires yara-rust crate)
/// Currently a placeholder — enable with the yara feature flag
pub fn scan_with_yara(_data: &[u8], _rules_path: &str) -> Vec<String> {
    // Placeholder: YARA integration requires the yara-rust crate
    // which needs libyara system library
    vec![]
}
