use serde::Serialize;

/// Known Dart VM runtime stubs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum StubKind {
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
    CallToRuntime,
    ScheduleMicrotask,
    Unknown,
}

impl std::fmt::Display for StubKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StubKind::AllocateObject => write!(f, "AllocateObject"),
            StubKind::AllocateArray => write!(f, "AllocateArray"),
            StubKind::InstanceOf => write!(f, "InstanceOf"),
            StubKind::ThrowException => write!(f, "ThrowException"),
            StubKind::NullCheck => write!(f, "NullCheck"),
            StubKind::RangeCheck => write!(f, "RangeCheck"),
            StubKind::StackOverflowCheck => write!(f, "StackOverflowCheck"),
            StubKind::WriteBarrier => write!(f, "WriteBarrier"),
            StubKind::DeoptimizeLazyFromReturn => write!(f, "DeoptimizeLazyFromReturn"),
            StubKind::DeoptimizeLazyFromThrow => write!(f, "DeoptimizeLazyFromThrow"),
            StubKind::SubtypeCheck => write!(f, "SubtypeCheck"),
            StubKind::AssertAssignable => write!(f, "AssertAssignable"),
            StubKind::CallToRuntime => write!(f, "CallToRuntime"),
            StubKind::ScheduleMicrotask => write!(f, "ScheduleMicrotask"),
            StubKind::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Recognizes Dart VM stubs from instruction patterns
pub struct StubRecognizer {
    patterns: Vec<(StubKind, Vec<u8>)>,
}

impl StubRecognizer {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// Load patterns from a profile's stubs configuration
    pub fn load_from_profile(&mut self, stubs: &std::collections::HashMap<String, String>) {
        for (name, pattern_str) in stubs {
            let kind = match name.as_str() {
                "AllocateObjectStub" => StubKind::AllocateObject,
                "AllocateArrayStub" => StubKind::AllocateArray,
                "CallToRuntimeStub" => StubKind::CallToRuntime,
                "NullCheckStub" => StubKind::NullCheck,
                "TypeCheckStub" => StubKind::SubtypeCheck,
                "InstanceOfStub" => StubKind::InstanceOf,
                "ThrowExceptionStub" => StubKind::ThrowException,
                "RangeCheckStub" => StubKind::RangeCheck,
                "WriteBarrierStub" => StubKind::WriteBarrier,
                _ => StubKind::Unknown,
            };

            if let Some(bytes) = parse_pattern_string(pattern_str) {
                self.patterns.push((kind, bytes));
            }
        }
    }

    /// Try to recognize a stub from instruction bytes
    pub fn recognize(&self, instruction_bytes: &[u8]) -> Option<StubKind> {
        for (kind, pattern) in &self.patterns {
            if instruction_bytes.len() >= pattern.len()
                && &instruction_bytes[..pattern.len()] == pattern.as_slice()
            {
                return Some(*kind);
            }
        }
        None
    }

    /// Recognize by address (for pre-computed stub addresses)
    pub fn recognize_by_address(&self, _addr: u64) -> Option<StubKind> {
        // In a full implementation, this would map known stub addresses
        None
    }
}

impl Default for StubRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse "pattern:aarch64:d10043ff..." into bytes
fn parse_pattern_string(s: &str) -> Option<Vec<u8>> {
    let hex_part = s.rsplit(':').next()?;
    let mut bytes = Vec::new();
    let mut chars = hex_part.chars();
    while let (Some(hi), Some(lo)) = (chars.next(), chars.next()) {
        let byte = u8::from_str_radix(&format!("{}{}", hi, lo), 16).ok()?;
        bytes.push(byte);
    }
    if bytes.is_empty() {
        None
    } else {
        Some(bytes)
    }
}
