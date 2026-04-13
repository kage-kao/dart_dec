#[cfg(test)]
mod tests {
    use crate::*;
    use crate::json::{JsonOutput, StringEntry};

    fn make_test_meta() -> OutputMeta {
        OutputMeta {
            tool: "dart_dec".to_string(),
            version: "0.1.0".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            input_file: "test.so".to_string(),
            input_sha256: "abc123".to_string(),
            dart_version: dart_dec_core::DartVersion {
                major: 3, minor: 2, patch: 0,
                channel: dart_dec_core::Channel::Stable,
                sdk_hash: None,
                detection_method: dart_dec_core::DetectionMethod::VersionString,
                confidence: dart_dec_core::version::Confidence::High,
                raw_string: None,
            },
            architecture: "arm64".to_string(),
            analysis_time_ms: 1000,
        }
    }

    fn make_test_stats() -> OutputStats {
        OutputStats {
            total_classes: 100,
            total_functions: 500,
            total_strings: 1000,
            decompiled_functions: 450,
            failed_functions: 50,
            coverage_percent: 90.0,
        }
    }

    #[test]
    fn test_json_output() {
        let output = JsonOutput {
            meta: make_test_meta(),
            statistics: make_test_stats(),
            libraries: vec![],
            strings: vec![StringEntry { value: "hello".to_string(), refs_count: 1 }],
            security_findings: vec![],
        };

        let mut buf = Vec::new();
        json::write_json(&output, &mut buf).unwrap();
        let json_str = String::from_utf8(buf).unwrap();
        assert!(json_str.contains("dart_dec"));
        assert!(json_str.contains("hello"));
    }

    #[test]
    fn test_jsonl_output() {
        let output = JsonOutput {
            meta: make_test_meta(),
            statistics: make_test_stats(),
            libraries: vec![],
            strings: vec![],
            security_findings: vec![],
        };

        let mut buf = Vec::new();
        json::write_jsonl(&output, &mut buf).unwrap();
        let output_str = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_sqlite_create_database() {
        let path = std::path::PathBuf::from("/tmp/test_dart_dec.db");
        let conn = sqlite::create_database(&path).unwrap();
        sqlite::write_meta(&conn, &make_test_meta()).unwrap();

        let tool: String = conn.query_row(
            "SELECT value FROM meta WHERE key = 'tool'",
            [],
            |row: &rusqlite::Row| row.get(0),
        ).unwrap();
        assert_eq!(tool, "dart_dec");

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_sarif_report() {
        let report = sarif::new_report();
        assert_eq!(report.version, "2.1.0");

        let mut buf = Vec::new();
        sarif::write_sarif(&report, &mut buf).unwrap();
        let json_str = String::from_utf8(buf).unwrap();
        assert!(json_str.contains("dart_dec"));
    }

    #[test]
    fn test_dart_codegen_empty() {
        let dir = std::path::PathBuf::from("/tmp/test_dart_dec_output");
        let result = dart_codegen::generate_dart_files(&[], &dir);
        assert!(result.is_ok());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn test_dart_codegen_with_class() {
        let dir = std::path::PathBuf::from("/tmp/test_dart_dec_output2");
        let ast = dart_dec_graph::AstNode::Return(Some(dart_dec_graph::AstExpr::Literal(
            dart_dec_graph::DartLiteral::Int(42),
        )));
        let libraries = vec![(
            "package:test/main.dart".to_string(),
            vec![(
                "MyClass".to_string(),
                vec![("getValue".to_string(), ast)],
            )],
        )];
        let result = dart_codegen::generate_dart_files(&libraries, &dir);
        assert!(result.is_ok());
        std::fs::remove_dir_all(dir).ok();
    }
}
