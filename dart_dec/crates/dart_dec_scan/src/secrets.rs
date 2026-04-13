use crate::{SecurityFinding, Severity};
use regex::Regex;

/// Scan strings for hardcoded secrets (API keys, tokens, URLs)
pub fn scan_secrets(strings: &[String]) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    
    let patterns = vec![
        ("AWS Access Key", r"AKIA[0-9A-Z]{16}", Severity::Critical),
        ("AWS Secret Key", r#"(?i)aws(.{0,20})?(?-i)['"][0-9a-zA-Z/+]{40}['"]"#, Severity::Critical),
        ("Google API Key", r"AIza[0-9A-Za-z\-_]{35}", Severity::High),
        ("Firebase URL", r"https://[a-z0-9-]+\.firebaseio\.com", Severity::Medium),
        ("Private Key", r"-----BEGIN (RSA |EC )?PRIVATE KEY-----", Severity::Critical),
        ("JWT Token", r"eyJ[A-Za-z0-9-_]+\.eyJ[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+", Severity::High),
        ("Slack Token", r"xox[bpors]-[0-9]{12}-[0-9]{12}-[a-zA-Z0-9]{24}", Severity::High),
        ("GitHub Token", r"gh[pousr]_[A-Za-z0-9_]{36}", Severity::High),
        ("HTTP URL (insecure)", r#"http://[^\s"']+"#, Severity::Low),
        ("Hardcoded Password", r#"(?i)(password|passwd|pwd)\s*[:=]\s*['"][^'"]{4,}['"]"#, Severity::High),
        ("Bearer Token", r"(?i)bearer\s+[a-z0-9\-._~+/]+=*", Severity::High),
    ];
    
    for string in strings {
        for (name, pattern, severity) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(string) {
                    findings.push(SecurityFinding {
                        rule_id: format!("SECRET_{}", name.to_uppercase().replace(' ', "_")),
                        severity: *severity,
                        description: format!("Possible {}", name),
                        evidence: truncate(string, 100),
                        location: None,
                    });
                }
            }
        }
    }
    
    findings
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max { format!("{}...", &s[..max]) } else { s.to_string() }
}
