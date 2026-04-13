use crate::ir::StubCallKind;
use dart_dec_snapshot::StubKind;

/// Resolves function addresses and pool offsets to known Dart VM stubs
pub struct StubResolver {
    address_map: ahash::AHashMap<u64, StubCallKind>,
    pool_offset_map: ahash::AHashMap<u64, StubCallKind>,
}

impl StubResolver {
    pub fn new() -> Self {
        Self {
            address_map: ahash::AHashMap::new(),
            pool_offset_map: ahash::AHashMap::new(),
        }
    }

    /// Register a known stub at a specific address
    pub fn register_stub(&mut self, address: u64, kind: StubCallKind) {
        self.address_map.insert(address, kind);
    }

    /// Register a stub accessible through a pool offset
    pub fn register_pool_stub(&mut self, offset: u64, kind: StubCallKind) {
        self.pool_offset_map.insert(offset, kind);
    }

    /// Try to resolve a call target address to a known stub
    pub fn resolve_address(&self, address: u64) -> Option<StubCallKind> {
        self.address_map.get(&address).copied()
    }

    /// Try to resolve a pool offset to a known stub
    pub fn resolve_pool_offset(&self, offset: u64) -> Option<StubCallKind> {
        self.pool_offset_map.get(&offset).copied()
    }

    /// Convert snapshot StubKind to IR StubCallKind
    pub fn from_snapshot_stub(kind: StubKind) -> StubCallKind {
        match kind {
            StubKind::AllocateObject => StubCallKind::AllocateObject,
            StubKind::AllocateArray => StubCallKind::AllocateArray,
            StubKind::InstanceOf => StubCallKind::InstanceOf,
            StubKind::ThrowException => StubCallKind::ThrowException,
            StubKind::NullCheck => StubCallKind::NullCheck,
            StubKind::RangeCheck => StubCallKind::RangeCheck,
            StubKind::StackOverflowCheck => StubCallKind::StackOverflowCheck,
            StubKind::WriteBarrier => StubCallKind::WriteBarrier,
            StubKind::DeoptimizeLazyFromReturn => StubCallKind::DeoptimizeLazyFromReturn,
            StubKind::DeoptimizeLazyFromThrow => StubCallKind::DeoptimizeLazyFromThrow,
            StubKind::SubtypeCheck => StubCallKind::SubtypeCheck,
            StubKind::AssertAssignable => StubCallKind::AssertAssignable,
            StubKind::CallToRuntime | StubKind::ScheduleMicrotask | StubKind::Unknown => {
                StubCallKind::AllocateObject // fallback
            }
        }
    }
}

impl Default for StubResolver {
    fn default() -> Self {
        Self::new()
    }
}
