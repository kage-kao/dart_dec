use crate::object_pool::ObjectPool;
use crate::types::*;
use dart_dec_profiles::DartProfile;
use tracing::debug;

/// Table of all recovered Dart classes
#[derive(Debug, Clone)]
pub struct ClassTable {
    pub classes: Vec<DartClass>,
    /// Map from SnapshotAddr to index in classes vec
    pub addr_to_idx: ahash::AHashMap<SnapshotAddr, usize>,
}

impl ClassTable {
    pub fn new() -> Self {
        Self {
            classes: Vec::new(),
            addr_to_idx: ahash::AHashMap::new(),
        }
    }

    pub fn get_by_addr(&self, addr: &SnapshotAddr) -> Option<&DartClass> {
        self.addr_to_idx.get(addr).map(|&idx| &self.classes[idx])
    }

    pub fn get_by_name(&self, name: &str) -> Option<&DartClass> {
        self.classes.iter().find(|c| c.name == name)
    }

    pub fn len(&self) -> usize {
        self.classes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }
}

/// Build a class table from the object pool
pub fn build_class_table(pool: &ObjectPool, _profile: &DartProfile) -> ClassTable {
    let mut table = ClassTable::new();

    // First pass: collect all classes
    for (addr, obj) in pool.iter() {
        if let DartObject::Class(class) = obj {
            let idx = table.classes.len();
            table.addr_to_idx.insert(*addr, idx);
            table.classes.push(class.clone());
        }
    }

    // Second pass: resolve names from string pool
    for class in &mut table.classes {
        // Try to resolve class name from string objects in the pool
        for (_str_addr, obj) in pool.iter() {
            if let DartObject::String(s) = obj {
                // Heuristic: look for strings that could be class names
                if !s.value.is_empty()
                    && s.value.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    && !s.value.contains(' ')
                    && s.value.len() < 256
                {
                    // This is a potential class name — matching is done by proximity
                    // in a real implementation, we'd resolve the name pointer
                }
            }
        }
    }

    // Third pass: resolve function and field references
    for (_addr, obj) in pool.iter() {
        match obj {
            DartObject::Function(func) => {
                if let Some(owner) = &func.owner_class {
                    if let Some(idx) = table.addr_to_idx.get(owner) {
                        table.classes[*idx].functions.push(func.addr);
                    }
                }
            }
            DartObject::Field(field) => {
                if let Some(owner) = &field.owner_class {
                    if let Some(idx) = table.addr_to_idx.get(owner) {
                        table.classes[*idx].fields.push(field.addr);
                    }
                }
            }
            _ => {}
        }
    }

    debug!("Built class table with {} classes", table.len());
    table
}
