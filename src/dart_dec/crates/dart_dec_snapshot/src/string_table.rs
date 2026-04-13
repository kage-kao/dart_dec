use crate::object_pool::ObjectPool;
use crate::types::*;
use tracing::debug;

/// Table of all extracted strings from the snapshot
#[derive(Debug, Clone)]
pub struct StringTable {
    pub strings: Vec<DartString>,
    pub addr_to_idx: ahash::AHashMap<SnapshotAddr, usize>,
}

impl StringTable {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            addr_to_idx: ahash::AHashMap::new(),
        }
    }

    pub fn get_by_addr(&self, addr: &SnapshotAddr) -> Option<&str> {
        self.addr_to_idx
            .get(addr)
            .map(|&idx| self.strings[idx].value.as_str())
    }

    pub fn find(&self, needle: &str) -> Vec<&DartString> {
        self.strings
            .iter()
            .filter(|s| s.value.contains(needle))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

/// Extract all strings from the object pool
pub fn extract_strings(pool: &ObjectPool, _raw_data: &[u8]) -> StringTable {
    let mut table = StringTable::new();

    for (addr, obj) in pool.iter() {
        if let DartObject::String(s) = obj {
            let idx = table.strings.len();
            table.addr_to_idx.insert(*addr, idx);
            table.strings.push(s.clone());
        }
    }

    debug!("Extracted {} strings from snapshot", table.len());
    table
}
