/// Integration test: full pipeline from IR to Dart code
use dart_dec_graph::*;
use dart_dec_lifter::*;

#[test]
fn test_full_pipeline_simple_function() {
    // Create IR for: int add(int a, int b) { return a + b; }
    let ir = vec![
        IR::BinOp {
            dst: Reg(2),
            op: BinOpKind::Add,
            lhs: Operand::Reg(Reg(0)),
            rhs: Operand::Reg(Reg(1)),
        },
        IR::Return(Some(Operand::Reg(Reg(2)))),
    ];

    // Build CFG
    let cfg = CFG::build(&ir);
    assert!(cfg.num_blocks() >= 1);

    // Compute dominators
    let dom_tree = DominatorTree::compute(&cfg);

    // Structure to AST
    let ast = structure_cfg(&cfg, &dom_tree);

    // Generate Dart code
    let code = generate_dart_code(&ast, 0);
    assert!(!code.is_empty());
    assert!(code.contains("return"));
}

#[test]
fn test_full_pipeline_if_else() {
    let ir = vec![
        IR::Assign(Reg(0), Operand::Imm(5)),
        IR::Compare {
            lhs: Operand::Reg(Reg(0)),
            rhs: Operand::Imm(10),
        },
        IR::Branch {
            condition: Condition::LessThan,
            true_target: 3,
            false_target: 5,
        },
        // true branch
        IR::Assign(Reg(1), Operand::Imm(1)),
        IR::Return(Some(Operand::Reg(Reg(1)))),
        // false branch
        IR::Assign(Reg(1), Operand::Imm(0)),
        IR::Return(Some(Operand::Reg(Reg(1)))),
    ];

    let cfg = CFG::build(&ir);
    let dom_tree = DominatorTree::compute(&cfg);
    let ast = structure_cfg(&cfg, &dom_tree);
    let code = generate_dart_code(&ast, 0);

    assert!(code.contains("if"));
}

#[test]
fn test_full_pipeline_dot_export() {
    let ir = vec![
        IR::Assign(Reg(0), Operand::Imm(0)),
        IR::Return(Some(Operand::Reg(Reg(0)))),
    ];

    let cfg = CFG::build(&ir);
    let dot = cfg.to_dot();

    assert!(dot.starts_with("digraph CFG"));
    assert!(dot.contains("BB"));
}

#[test]
fn test_security_scan_integration() {
    let test_strings = vec![
        "normal text".to_string(),
        "AKIAIOSFODNN7EXAMPLE".to_string(), // AWS key
        "http://insecure.example.com".to_string(), // HTTP URL
        "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij".to_string(), // GitHub token
    ];

    let findings = dart_dec_scan::scan_all(&test_strings, &[]);
    assert!(findings.len() >= 3); // At least AWS + HTTP + GitHub
}

#[test]
fn test_profile_roundtrip() {
    let resolver = dart_dec_profiles::ProfileResolver::new();
    let profile = resolver.resolve(3, 2, 0).unwrap();

    // Serialize to JSON and back
    let json = serde_json::to_string(profile).unwrap();
    let parsed: dart_dec_profiles::DartProfile = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.version, "3.2.0");
    assert_eq!(parsed.class_id("OneByteString"), Some(78));
}

#[test]
fn test_disasm_lift_roundtrip() {
    use dart_dec_disasm::{Arm64Disassembler, Disassembler};

    let disasm = Arm64Disassembler::new().unwrap();

    // ARM64: MOV X0, #42; RET
    let code = [
        0x40, 0x05, 0x80, 0xd2, // MOV X0, #42
        0xc0, 0x03, 0x5f, 0xd6, // RET
    ];

    let insns = disasm.disassemble(&code, 0x1000).unwrap();
    assert!(insns.len() >= 2);

    let pool_resolver = dart_dec_lifter::PoolResolver::new();
    let stub_resolver = dart_dec_lifter::StubResolver::new();

    let ir = dart_dec_lifter::lift_instructions(
        &insns,
        dart_dec_core::Architecture::Arm64,
        &pool_resolver,
        &stub_resolver,
    )
    .unwrap();

    assert!(!ir.is_empty());
    // Should contain at least an assignment and a return
    assert!(ir.iter().any(|i| matches!(i, IR::Return(_))));
}
