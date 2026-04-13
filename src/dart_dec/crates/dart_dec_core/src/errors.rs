use thiserror::Error;

/// Core errors for binary parsing and version detection
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Not a valid ELF file: {0}")]
    InvalidElf(String),

    #[error("Not a valid Mach-O file: {0}")]
    InvalidMachO(String),

    #[error("Not a valid PE file: {0}")]
    InvalidPe(String),

    #[error("Unknown binary format (magic bytes: {magic:?})")]
    UnknownFormat { magic: [u8; 4] },

    #[error("Section '{name}' not found at expected offset")]
    SectionNotFound { name: String },

    #[error("Section '{name}' has zero size")]
    SectionEmpty { name: String },

    #[error("Dart VM version '{version}' is not supported")]
    UnsupportedVersion { version: String },

    #[error("Could not detect Dart VM version from binary")]
    VersionNotDetected,

    #[error("Architecture not supported: {0}")]
    UnsupportedArchitecture(String),

    #[error("File too small: expected at least {expected} bytes, got {actual}")]
    FileTooSmall { expected: usize, actual: usize },

    #[error("Invalid offset: {offset} exceeds file size {file_size}")]
    InvalidOffset { offset: usize, file_size: usize },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Goblin parse error: {0}")]
    GoblinError(String),
}

impl From<goblin::error::Error> for CoreError {
    fn from(e: goblin::error::Error) -> Self {
        CoreError::GoblinError(e.to_string())
    }
}
