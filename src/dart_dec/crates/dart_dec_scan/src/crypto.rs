use crate::{SecurityFinding, Severity};

/// Scan for weak cryptographic primitives
pub fn scan_weak_crypto(function_names: &[String]) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let weak_patterns = vec![
        ("MD5", "Use of MD5 hash (insecure)"),
        ("SHA1", "Use of SHA-1 hash (deprecated)"),
        ("DES", "Use of DES encryption (insecure)"),
        ("RC4", "Use of RC4 cipher (insecure)"),
        ("ECB", "Use of ECB mode (insecure)"),
    ];
    
    for func in function_names {
        for (pattern, desc) in &weak_patterns {
            if func.to_uppercase().contains(pattern) {
                findings.push(SecurityFinding {
                    rule_id: format!("CRYPTO_WEAK_{}", pattern),
                    severity: Severity::Medium,
                    description: desc.to_string(),
                    evidence: func.clone(),
                    location: None,
                });
            }
        }
    }
    
    findings
}
