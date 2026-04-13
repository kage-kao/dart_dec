#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_scan_aws_key() {
        let strings = vec!["AKIAIOSFODNN7EXAMPLE".to_string()];
        let findings = secrets::scan_secrets(&strings);
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id.contains("AWS")));
    }

    #[test]
    fn test_scan_google_api_key() {
        let strings = vec!["AIzaSyA1234567890abcdefghijklmnopqrstuv".to_string()];
        let findings = secrets::scan_secrets(&strings);
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id.contains("GOOGLE")));
    }

    #[test]
    fn test_scan_http_url() {
        let strings = vec!["http://insecure-api.example.com/data".to_string()];
        let findings = secrets::scan_secrets(&strings);
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id.contains("HTTP")));
    }

    #[test]
    fn test_scan_no_findings() {
        let strings = vec!["Hello World".to_string(), "normal text".to_string()];
        let findings = secrets::scan_secrets(&strings);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_scan_jwt_token() {
        let strings = vec![
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U".to_string()
        ];
        let findings = secrets::scan_secrets(&strings);
        assert!(findings.iter().any(|f| f.rule_id.contains("JWT")));
    }

    #[test]
    fn test_scan_weak_crypto_md5() {
        let functions = vec!["computeMD5Hash".to_string()];
        let findings = crypto::scan_weak_crypto(&functions);
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id.contains("MD5")));
    }

    #[test]
    fn test_scan_weak_crypto_sha1() {
        let functions = vec!["sha1Digest".to_string()];
        let findings = crypto::scan_weak_crypto(&functions);
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_scan_no_weak_crypto() {
        let functions = vec!["sha256Hash".to_string(), "aesEncrypt".to_string()];
        let findings = crypto::scan_weak_crypto(&functions);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_scan_all() {
        let strings = vec!["AKIAIOSFODNN7EXAMPLE".to_string()];
        let functions = vec!["computeMD5Hash".to_string()];
        let findings = scan_all(&strings, &functions);
        assert!(findings.len() >= 2);
    }

    #[test]
    fn test_severity_ordering() {
        assert_ne!(Severity::Critical, Severity::Low);
        assert_ne!(Severity::High, Severity::Info);
    }

    #[test]
    fn test_permissions_parsing() {
        let manifest = r#"
            <uses-permission android:name="android.permission.INTERNET"/>
            <uses-permission android:name="android.permission.CAMERA"/>
        "#;
        let perms = permissions::analyze_permissions(manifest);
        assert_eq!(perms.len(), 2);
        assert!(perms.iter().any(|p| p.contains("INTERNET")));
        assert!(perms.iter().any(|p| p.contains("CAMERA")));
    }

    #[test]
    fn test_github_token_detection() {
        let strings = vec!["ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij".to_string()];
        let findings = secrets::scan_secrets(&strings);
        assert!(findings.iter().any(|f| f.rule_id.contains("GITHUB")));
    }

    #[test]
    fn test_private_key_detection() {
        let strings = vec!["-----BEGIN RSA PRIVATE KEY-----".to_string()];
        let findings = secrets::scan_secrets(&strings);
        assert!(findings.iter().any(|f| f.severity == Severity::Critical));
    }
}
