use dart_dec_snapshot::types::SnapshotAddr;
use serde::Serialize;

/// Abstract register (not tied to any architecture)
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize)]
pub struct Reg(pub u16);

impl std::fmt::Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r{}", self.0)
    }
}

/// Block identifier for CFG
pub type BlockId = u32;

/// IR Operand
#[derive(Debug, Clone, Serialize)]
pub enum Operand {
    Reg(Reg),
    Imm(i64),
    PoolObject(SnapshotAddr),
    StackSlot(i32),
    FieldAccess {
        base: Reg,
        offset: u32,
        field_name: Option<String>,
    },
}

/// Branch condition
#[derive(Debug, Clone, Serialize)]
pub enum Condition {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessOrEqual,
    GreaterOrEqual,
    True(Reg),
    False(Reg),
    TypeCheck(Reg, DartTypeRef),
}

/// Type of function call
#[derive(Debug, Clone, Serialize)]
pub enum CallKind {
    Direct(SnapshotAddr),
    Virtual(u32),
    DynamicDispatch(String),
    Stub(StubCallKind),
    Closure(Reg),
}

/// Dart VM stub call types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum StubCallKind {
    AllocateObject,
    AllocateArray,
    InstanceOf,
    ThrowException,
    NullCheck,
    RangeCheck,
    StackOverflowCheck,
    WriteBarrier,
    DeoptimizeLazyFromReturn,
    DeoptimizeLazyFromThrow,
    SubtypeCheck,
    AssertAssignable,
}

/// Binary operation kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Shr,
    Shl,
    Asr,
    And,
    Or,
    Xor,
}

/// Unary operation kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum UnaryOpKind {
    Neg,
    Not,
    BitNot,
}

/// Dart type reference for type checks
#[derive(Debug, Clone, Serialize)]
pub struct DartTypeRef {
    pub name: String,
    pub is_nullable: bool,
}

/// Main IR instruction set
#[derive(Debug, Clone, Serialize)]
pub enum IR {
    /// Assignment: dst = src
    Assign(Reg, Operand),

    /// Binary operation: dst = lhs op rhs
    BinOp {
        dst: Reg,
        op: BinOpKind,
        lhs: Operand,
        rhs: Operand,
    },

    /// Unary operation: dst = op src
    UnaryOp {
        dst: Reg,
        op: UnaryOpKind,
        src: Operand,
    },

    /// Function call: dst = call(args...)
    Call {
        dst: Option<Reg>,
        kind: CallKind,
        args: Vec<Operand>,
    },

    /// Load from Object Pool
    LoadPool {
        dst: Reg,
        addr: SnapshotAddr,
        resolved: Option<String>,
    },

    /// Field read: dst = base.field
    LoadField {
        dst: Reg,
        base: Reg,
        offset: u32,
        field_name: Option<String>,
    },

    /// Field write: base.field = value
    StoreField {
        base: Reg,
        offset: u32,
        value: Operand,
        field_name: Option<String>,
    },

    /// Conditional branch
    Branch {
        condition: Condition,
        true_target: BlockId,
        false_target: BlockId,
    },

    /// Unconditional jump
    Jump(BlockId),

    /// Return from function
    Return(Option<Operand>),

    /// Throw exception
    Throw(Operand),

    /// Phi function (after SSA transformation)
    Phi {
        dst: Reg,
        sources: Vec<(BlockId, Reg)>,
    },

    /// Dart null check (from NullCheckStub)
    NullCheck(Reg),

    /// Dart type check (is / as)
    TypeCheck {
        src: Reg,
        target_type: DartTypeRef,
        is_cast: bool,
    },

    /// Compare instruction (sets flags for subsequent branch)
    Compare {
        lhs: Operand,
        rhs: Operand,
    },

    /// Unrecognized instruction (graceful degradation)
    Unknown {
        address: u64,
        raw_asm: String,
    },
}

impl IR {
    /// Get the destination register if this IR writes to one
    pub fn dest_reg(&self) -> Option<Reg> {
        match self {
            IR::Assign(r, _) => Some(*r),
            IR::BinOp { dst, .. } => Some(*dst),
            IR::UnaryOp { dst, .. } => Some(*dst),
            IR::Call { dst, .. } => *dst,
            IR::LoadPool { dst, .. } => Some(*dst),
            IR::LoadField { dst, .. } => Some(*dst),
            IR::Phi { dst, .. } => Some(*dst),
            _ => None,
        }
    }

    /// Check if this IR is a terminator (ends a basic block)
    pub fn is_terminator(&self) -> bool {
        matches!(
            self,
            IR::Branch { .. } | IR::Jump(_) | IR::Return(_) | IR::Throw(_)
        )
    }
}
