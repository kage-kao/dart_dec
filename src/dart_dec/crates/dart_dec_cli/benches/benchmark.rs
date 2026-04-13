use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_disassemble_arm64(c: &mut Criterion) {
    use dart_dec_disasm::{Arm64Disassembler, Disassembler};

    let disasm = Arm64Disassembler::new().unwrap();

    // Generate a series of NOP instructions
    let mut code = Vec::new();
    for _ in 0..1000 {
        code.extend_from_slice(&[0x1f, 0x20, 0x03, 0xd5]); // NOP
    }

    c.bench_function("disasm_arm64_1000_nops", |b| {
        b.iter(|| {
            let result = disasm.disassemble(black_box(&code), 0x1000);
            black_box(result.unwrap());
        })
    });
}

fn bench_disassemble_x86_64(c: &mut Criterion) {
    use dart_dec_disasm::{X86_64Disassembler, Disassembler};

    let disasm = X86_64Disassembler::new(dart_dec_core::Architecture::X86_64).unwrap();

    let mut code = Vec::new();
    for _ in 0..1000 {
        code.push(0x90); // NOP
    }

    c.bench_function("disasm_x86_64_1000_nops", |b| {
        b.iter(|| {
            let result = disasm.disassemble(black_box(&code), 0x1000);
            black_box(result.unwrap());
        })
    });
}

fn bench_cfg_build(c: &mut Criterion) {
    use dart_dec_lifter::*;

    // Build a moderate-sized IR sequence
    let mut ir = Vec::new();
    for i in 0..100 {
        ir.push(IR::Assign(Reg(i as u16), Operand::Imm(i)));
    }
    ir.push(IR::Return(Some(Operand::Reg(Reg(0)))));

    c.bench_function("cfg_build_100_instrs", |b| {
        b.iter(|| {
            let cfg = dart_dec_graph::CFG::build(black_box(&ir));
            black_box(cfg);
        })
    });
}

fn bench_security_scan(c: &mut Criterion) {
    let strings: Vec<String> = (0..1000)
        .map(|i| format!("normal_string_{}", i))
        .collect();

    c.bench_function("scan_secrets_1000_strings", |b| {
        b.iter(|| {
            let findings = dart_dec_scan::secrets::scan_secrets(black_box(&strings));
            black_box(findings);
        })
    });
}

fn bench_profile_resolve(c: &mut Criterion) {
    let resolver = dart_dec_profiles::ProfileResolver::new();

    c.bench_function("profile_resolve_exact", |b| {
        b.iter(|| {
            let profile = resolver.resolve(black_box(3), black_box(2), black_box(0));
            black_box(profile);
        })
    });

    c.bench_function("profile_resolve_fuzzy", |b| {
        b.iter(|| {
            let profile = resolver.resolve(black_box(3), black_box(3), black_box(1));
            black_box(profile);
        })
    });
}

fn bench_ast_codegen(c: &mut Criterion) {
    use dart_dec_graph::*;

    let ast = AstNode::Block(vec![
        AstNode::VarDecl {
            name: "x".to_string(),
            dart_type: Some("int".to_string()),
            value: Some(AstExpr::Literal(DartLiteral::Int(42))),
            is_final: true,
            is_late: false,
        },
        AstNode::If {
            condition: AstExpr::BinaryOp(
                Box::new(AstExpr::Variable("x".to_string())),
                BinOp::GreaterThan,
                Box::new(AstExpr::Literal(DartLiteral::Int(0))),
            ),
            then_body: Box::new(AstNode::Return(Some(AstExpr::Variable("x".to_string())))),
            else_body: Some(Box::new(AstNode::Return(Some(AstExpr::Literal(DartLiteral::Int(0)))))),
        },
    ]);

    c.bench_function("ast_codegen_if_else", |b| {
        b.iter(|| {
            let code = generate_dart_code(black_box(&ast), 0);
            black_box(code);
        })
    });
}

criterion_group!(
    benches,
    bench_disassemble_arm64,
    bench_disassemble_x86_64,
    bench_cfg_build,
    bench_security_scan,
    bench_profile_resolve,
    bench_ast_codegen,
);
criterion_main!(benches);
