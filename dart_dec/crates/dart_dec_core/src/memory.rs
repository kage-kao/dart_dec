use crate::errors::CoreError;
use crate::BinaryFormat;
use memmap2::MmapOptions;
use std::fs::File;
use std::path::Path;

/// Memory-mapped binary file for zero-copy access
pub struct BinaryFile {
    mmap: memmap2::Mmap,
    format: BinaryFormat,
    file_size: usize,
    sha256: String,
}

impl BinaryFile {
    /// Open and memory-map a binary file, detecting its format from magic bytes
    pub fn open(path: &Path) -> Result<Self, CoreError> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let file_size = metadata.len() as usize;

        if file_size < 4 {
            return Err(CoreError::FileTooSmall {
                expected: 4,
                actual: file_size,
            });
        }

        let mmap = unsafe { MmapOptions::new().map(&file)? };

        // Detect format from magic bytes
        let magic: [u8; 4] = [mmap[0], mmap[1], mmap[2], mmap[3]];
        let format = match magic {
            [0x7f, b'E', b'L', b'F'] => BinaryFormat::Elf,
            [0xfe, 0xed, 0xfa, 0xce] | [0xce, 0xfa, 0xed, 0xfe] => BinaryFormat::MachO,
            [0xfe, 0xed, 0xfa, 0xcf] | [0xcf, 0xfa, 0xed, 0xfe] => BinaryFormat::MachO,
            [b'M', b'Z', _, _] => BinaryFormat::Pe,
            _ => return Err(CoreError::UnknownFormat { magic }),
        };

        // Compute SHA-256 hash
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&mmap[..]);
        let sha256 = hex::encode(hasher.finalize());

        Ok(Self {
            mmap,
            format,
            file_size,
            sha256,
        })
    }

    /// Get the binary format
    pub fn format(&self) -> BinaryFormat {
        self.format
    }

    /// Get the total file size
    pub fn file_size(&self) -> usize {
        self.file_size
    }

    /// Get the SHA-256 hash of the file
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Zero-copy access to a slice of data
    pub fn slice(&self, offset: usize, len: usize) -> Result<&[u8], CoreError> {
        let end = offset.checked_add(len).ok_or(CoreError::InvalidOffset {
            offset,
            file_size: self.file_size,
        })?;
        if end > self.file_size {
            return Err(CoreError::InvalidOffset {
                offset: end,
                file_size: self.file_size,
            });
        }
        Ok(&self.mmap[offset..end])
    }

    /// Get the entire file data
    pub fn data(&self) -> &[u8] {
        &self.mmap[..]
    }

    /// Search for a byte pattern in the file using SIMD-accelerated memchr
    pub fn find_pattern(&self, pattern: &[u8]) -> Option<usize> {
        if pattern.is_empty() {
            return None;
        }
        memchr::memmem::find(&self.mmap[..], pattern)
    }

    /// Find all occurrences of a byte pattern
    pub fn find_all_patterns(&self, pattern: &[u8]) -> Vec<usize> {
        if pattern.is_empty() {
            return vec![];
        }
        memchr::memmem::find_iter(&self.mmap[..], pattern).collect()
    }
}
