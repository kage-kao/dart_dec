use dart_dec_snapshot::object_pool::ObjectPool;
use dart_dec_snapshot::types::*;

/// Resolves Object Pool references (PP-relative loads) to named objects
pub struct PoolResolver {
    pool_entries: ahash::AHashMap<u64, String>,
}

impl PoolResolver {
    pub fn new() -> Self {
        Self {
            pool_entries: ahash::AHashMap::new(),
        }
    }

    /// Build resolver from a parsed object pool
    pub fn from_object_pool(pool: &ObjectPool) -> Self {
        let mut entries = ahash::AHashMap::new();

        for (addr, obj) in pool.iter() {
            let name = match obj {
                DartObject::String(s) => format!("\"{}\"", truncate_str(&s.value, 64)),
                DartObject::Class(c) => format!("class:{}", c.name),
                DartObject::Function(f) => format!("func:{}", f.name),
                DartObject::Field(f) => format!("field:{}", f.name),
                DartObject::Mint(v) => format!("int:{}", v),
                DartObject::Double(v) => format!("double:{}", v),
                DartObject::Bool(v) => format!("bool:{}", v),
                DartObject::Null => "null".to_string(),
                DartObject::Type(t) => format!("type:{}", t.name),
                DartObject::Code(c) => format!("code@{}", c.addr),
                DartObject::Array(a) => format!("array[{}]", a.elements.len()),
                DartObject::Closure(c) => format!("closure@{}", c.function),
                DartObject::Record(r) => format!("record({})", r.num_fields),
                _ => format!("obj@{}", addr),
            };
            entries.insert(addr.0, name);
        }

        Self {
            pool_entries: entries,
        }
    }

    /// Resolve a pool offset to a human-readable name
    pub fn resolve_offset(&self, offset: u64) -> Option<String> {
        self.pool_entries.get(&offset).cloned()
    }

    /// Add a manual entry
    pub fn add_entry(&mut self, offset: u64, name: String) {
        self.pool_entries.insert(offset, name);
    }
}

impl Default for PoolResolver {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}
