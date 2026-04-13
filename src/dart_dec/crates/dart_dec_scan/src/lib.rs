//! dart_dec_scan — Security scanners: secrets, crypto, permissions.

pub mod crypto;
pub mod permissions;
pub mod secrets;
pub mod yara;

use serde::Serialize;

/// A security finding
#[derive(Debug, Clone, Serialize)]
pub struct SecurityFinding {
    pub rule_id: String,
    pub severity: Severity,
    pub description: String,
    pub evidence: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Run all security scanners
pub fn scan_all(strings: &[String], functions: &[String]) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    findings.extend(secrets::scan_secrets(strings));
    findings.extend(crypto::scan_weak_crypto(functions));
    findings
}

#[cfg(test)]
mod tests;
