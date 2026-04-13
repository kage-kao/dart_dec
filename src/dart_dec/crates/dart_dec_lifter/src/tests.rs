#[cfg(test)]
mod tests {
    use crate::*;
    use crate::ir::*;
    use crate::pool_resolver::PoolResolver;
    use crate::stub_resolver::StubResolver;
    use dart_dec_snapshot::types::SnapshotAddr;

    #[test]
    fn test_pool_resolver_new() {
        let resolver = PoolResolver::new();
        assert!(resolver.resolve_offset(0).is_none());
    }

    #[test]
    fn test_pool_resolver_add_entry() {
        let mut resolver = PoolResolver::new();
        resolver.add_entry(0x100, "test_string".to_string());
        assert_eq!(resolver.resolve_offset(0x100), Some("test_string".to_string()));
        assert!(resolver.resolve_offset(0x200).is_none());
    }

    #[test]
    fn test_stub_resolver_new() {
        let resolver = StubResolver::new();
        assert!(resolver.resolve_address(0).is_none());
    }

    #[test]
    fn test_stub_resolver_register() {
        let mut resolver = StubResolver::new();
        resolver.register_stub(0x1000, StubCallKind::NullCheck);
        assert_eq!(resolver.resolve_address(0x1000), Some(StubCallKind::NullCheck));
        assert!(resolver.resolve_address(0x2000).is_none());
    }

    #[test]
    fn test_stub_resolver_pool() {
        let mut resolver = StubResolver::new();
        resolver.register_pool_stub(0x40, StubCallKind::AllocateObject);
        assert_eq!(resolver.resolve_pool_offset(0x40), Some(StubCallKind::AllocateObject));
    }

    #[test]
    fn test_ir_terminator() {
        assert!(IR::Return(None).is_terminator());
        assert!(IR::Jump(0).is_terminator());
        assert!(IR::Throw(Operand::Imm(0)).is_terminator());
        assert!(!IR::Assign(Reg(0), Operand::Imm(0)).is_terminator());
    }

    #[test]
    fn test_ir_dest_reg() {
        let ir = IR::Assign(Reg(5), Operand::Imm(42));
        assert_eq!(ir.dest_reg(), Some(Reg(5)));

        let ir = IR::Return(None);
        assert_eq!(ir.dest_reg(), None);
    }

    #[test]
    fn test_lift_arm64_basic() {
        use dart_dec_disasm::{Arm64Disassembler, Disassembler};

        let disasm = Arm64Disassembler::new().unwrap();
        // RET instruction
        let code = [0xc0, 0x03, 0x5f, 0xd6];
        let insns = disasm.disassemble(&code, 0x1000).unwrap();

        let pool_resolver = PoolResolver::new();
        let stub_resolver = StubResolver::new();

        let ir = lift_instructions(
            &insns,
            dart_dec_core::Architecture::Arm64,
            &pool_resolver,
            &stub_resolver,
        ).unwrap();

        assert!(!ir.is_empty());
        assert!(ir.iter().any(|i| matches!(i, IR::Return(_))));
    }

    #[test]
    fn test_lift_x86_ret() {
        use dart_dec_disasm::{X86_64Disassembler, Disassembler};

        let disasm = X86_64Disassembler::new(dart_dec_core::Architecture::X86_64).unwrap();
        let code = [0xc3]; // RET
        let insns = disasm.disassemble(&code, 0x1000).unwrap();

        let pool_resolver = PoolResolver::new();
        let stub_resolver = StubResolver::new();

        let ir = lift_instructions(
            &insns,
            dart_dec_core::Architecture::X86_64,
            &pool_resolver,
            &stub_resolver,
        ).unwrap();

        assert!(!ir.is_empty());
        assert!(ir.iter().any(|i| matches!(i, IR::Return(_))));
    }

    #[test]
    fn test_reg_display() {
        assert_eq!(format!("{}", Reg(0)), "r0");
        assert_eq!(format!("{}", Reg(27)), "r27");
    }
}
