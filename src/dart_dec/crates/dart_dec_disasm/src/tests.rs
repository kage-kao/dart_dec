#[cfg(test)]
mod tests {
    use crate::*;
    use crate::common::*;
    use dart_dec_core::Architecture;

    #[test]
    fn test_create_arm64_disassembler() {
        let disasm = create_disassembler(Architecture::Arm64);
        assert!(disasm.is_ok());
        assert_eq!(disasm.unwrap().architecture(), Architecture::Arm64);
    }

    #[test]
    fn test_create_arm32_disassembler() {
        let disasm = create_disassembler(Architecture::Arm32);
        assert!(disasm.is_ok());
    }

    #[test]
    fn test_create_x86_64_disassembler() {
        let disasm = create_disassembler(Architecture::X86_64);
        assert!(disasm.is_ok());
    }

    #[test]
    fn test_arm64_disassemble_nop() {
        let disasm = Arm64Disassembler::new().unwrap();
        // ARM64 NOP = 0xd503201f
        let code = [0x1f, 0x20, 0x03, 0xd5];
        let result = disasm.disassemble(&code, 0x1000);
        assert!(result.is_ok());
        let insns = result.unwrap();
        assert_eq!(insns.len(), 1);
        assert_eq!(insns[0].mnemonic, "nop");
        assert!(!insns[0].is_branch);
        assert!(!insns[0].is_call);
        assert!(!insns[0].is_return);
    }

    #[test]
    fn test_arm64_disassemble_ret() {
        let disasm = Arm64Disassembler::new().unwrap();
        // ARM64 RET = 0xd65f03c0
        let code = [0xc0, 0x03, 0x5f, 0xd6];
        let result = disasm.disassemble(&code, 0x1000);
        assert!(result.is_ok());
        let insns = result.unwrap();
        assert_eq!(insns.len(), 1);
        assert_eq!(insns[0].mnemonic, "ret");
        assert!(insns[0].is_return);
    }

    #[test]
    fn test_arm64_disassemble_mov() {
        let disasm = Arm64Disassembler::new().unwrap();
        // MOV X0, #42 = 0xd2800540
        let code = [0x40, 0x05, 0x80, 0xd2];
        let result = disasm.disassemble(&code, 0x1000);
        assert!(result.is_ok());
        let insns = result.unwrap();
        assert!(!insns.is_empty());
    }

    #[test]
    fn test_x86_64_disassemble_nop() {
        let disasm = X86_64Disassembler::new(Architecture::X86_64).unwrap();
        let code = [0x90]; // NOP
        let result = disasm.disassemble(&code, 0x1000);
        assert!(result.is_ok());
        let insns = result.unwrap();
        assert_eq!(insns.len(), 1);
        assert_eq!(insns[0].mnemonic, "nop");
    }

    #[test]
    fn test_x86_64_disassemble_ret() {
        let disasm = X86_64Disassembler::new(Architecture::X86_64).unwrap();
        let code = [0xc3]; // RET
        let result = disasm.disassemble(&code, 0x1000);
        assert!(result.is_ok());
        let insns = result.unwrap();
        assert_eq!(insns[0].mnemonic, "ret");
        assert!(insns[0].is_return);
    }

    #[test]
    fn test_x86_64_disassemble_call() {
        let disasm = X86_64Disassembler::new(Architecture::X86_64).unwrap();
        // CALL +5 (relative)
        let code = [0xe8, 0x00, 0x00, 0x00, 0x00];
        let result = disasm.disassemble(&code, 0x1000);
        assert!(result.is_ok());
        let insns = result.unwrap();
        assert_eq!(insns[0].mnemonic, "call");
        assert!(insns[0].is_call);
    }

    #[test]
    fn test_instruction_control_flow() {
        let insn = Instruction {
            address: 0x1000,
            size: 4,
            mnemonic: "bl".to_string(),
            op_str: "#0x2000".to_string(),
            bytes: vec![],
            operands: vec![],
            is_branch: false,
            is_call: true,
            is_return: false,
            branch_target: Some(0x2000),
        };
        assert!(insn.is_control_flow());
    }

    #[test]
    fn test_instruction_to_string() {
        let insn = Instruction {
            address: 0x1000,
            size: 4,
            mnemonic: "mov".to_string(),
            op_str: "x0, #42".to_string(),
            bytes: vec![],
            operands: vec![],
            is_branch: false,
            is_call: false,
            is_return: false,
            branch_target: None,
        };
        assert_eq!(insn.to_string_repr(), "0x00001000: mov x0, #42");
    }

    #[test]
    fn test_arm32_disassemble() {
        let disasm = Arm32Disassembler::new().unwrap();
        // Thumb NOP = 0xbf00
        let code = [0x00, 0xbf];
        let result = disasm.disassemble(&code, 0x1000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_input() {
        let disasm = Arm64Disassembler::new().unwrap();
        let result = disasm.disassemble(&[], 0x1000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
