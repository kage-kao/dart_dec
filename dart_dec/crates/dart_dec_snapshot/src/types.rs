use serde::Serialize;

/// Address in the snapshot's virtual space
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize)]
pub struct SnapshotAddr(pub u64);

impl std::fmt::Display for SnapshotAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

/// Universal object tag from Dart VM
#[derive(Debug, Clone, Serialize)]
pub struct ObjectTag {
    pub class_id: u16,
    pub size: u32,
    pub is_canonical: bool,
    pub is_old: bool,
}

/// Hierarchy of Dart objects recovered from the snapshot
#[derive(Debug, Clone, Serialize)]
pub enum DartObject {
    Class(DartClass),
    Function(DartFunction),
    Field(DartField),
    Code(DartCode),
    String(DartString),
    Array(DartArray),
    Mint(i64),
    Double(f64),
    Bool(bool),
    Null,
    Type(DartType),
    Closure(DartClosure),
    Record(DartRecord),
    SentinelObject,
    Unknown { class_id: u16, raw: Vec<u8> },
}

#[derive(Debug, Clone, Serialize)]
pub struct DartClass {
    pub addr: SnapshotAddr,
    pub name: String,
    pub library: String,
    pub super_class: Option<SnapshotAddr>,
    pub interfaces: Vec<SnapshotAddr>,
    pub fields: Vec<SnapshotAddr>,
    pub functions: Vec<SnapshotAddr>,
    pub type_parameters: Vec<String>,
    pub is_abstract: bool,
    pub is_sealed: bool,
    pub is_mixin: bool,
    pub is_enum: bool,
    pub class_id: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct DartFunction {
    pub addr: SnapshotAddr,
    pub name: String,
    pub owner_class: Option<SnapshotAddr>,
    pub code_addr: SnapshotAddr,
    pub kind: FunctionKind,
    pub is_static: bool,
    pub is_async: bool,
    pub is_generator: bool,
    pub parameter_names: Vec<String>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum FunctionKind {
    RegularFunction,
    Getter,
    Setter,
    Constructor,
    FactoryConstructor,
    ImplicitGetter,
    ImplicitSetter,
    ClosureFunction,
    AsyncClosure,
}

impl std::fmt::Display for FunctionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionKind::RegularFunction => write!(f, "regular"),
            FunctionKind::Getter => write!(f, "getter"),
            FunctionKind::Setter => write!(f, "setter"),
            FunctionKind::Constructor => write!(f, "constructor"),
            FunctionKind::FactoryConstructor => write!(f, "factory"),
            FunctionKind::ImplicitGetter => write!(f, "implicit_getter"),
            FunctionKind::ImplicitSetter => write!(f, "implicit_setter"),
            FunctionKind::ClosureFunction => write!(f, "closure"),
            FunctionKind::AsyncClosure => write!(f, "async_closure"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DartField {
    pub addr: SnapshotAddr,
    pub name: String,
    pub owner_class: Option<SnapshotAddr>,
    pub field_type: Option<String>,
    pub is_static: bool,
    pub is_final: bool,
    pub is_const: bool,
    pub is_late: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DartCode {
    pub addr: SnapshotAddr,
    pub instructions_offset: u64,
    pub instructions_size: u32,
    pub object_pool_addr: Option<SnapshotAddr>,
    pub pc_descriptors: Vec<PcDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PcDescriptor {
    pub pc_offset: u32,
    pub deopt_id: u32,
    pub token_pos: i32,
    pub kind: PcDescriptorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PcDescriptorKind {
    Deopt,
    IcCall,
    UnoptStaticCall,
    RuntimeCall,
    OsrEntry,
    Rewind,
    BSSReloc,
    Other(u8),
}

#[derive(Debug, Clone, Serialize)]
pub struct DartString {
    pub addr: SnapshotAddr,
    pub value: String,
    pub is_one_byte: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DartArray {
    pub addr: SnapshotAddr,
    pub elements: Vec<SnapshotAddr>,
    pub is_immutable: bool,
    pub type_args: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DartType {
    pub addr: SnapshotAddr,
    pub name: String,
    pub is_nullable: bool,
    pub type_arguments: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DartClosure {
    pub addr: SnapshotAddr,
    pub function: SnapshotAddr,
    pub context: Option<SnapshotAddr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DartRecord {
    pub addr: SnapshotAddr,
    pub num_fields: u32,
    pub field_names: Vec<String>,
    pub field_values: Vec<SnapshotAddr>,
}
