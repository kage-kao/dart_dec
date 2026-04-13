use crate::errors::CoreError;
use crate::memory::BinaryFile;
use crate::sections::ParsedBinary;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Dart VM release channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    Stable,
    Beta,
    Dev,
    Unknown,
}

/// How the version was detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DetectionMethod {
    /// Found version string directly (e.g. "3.2.0 (stable)")
    VersionString,
    /// Identified by SDK hash
    SdkHash,
    /// Heuristic based on struct sizes/fingerprinting
    StructFingerprint,
    /// Compared stub code patterns
    StubComparison,
}

/// Detected Dart VM version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DartVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub channel: Channel,
    pub sdk_hash: Option<String>,
    pub detection_method: DetectionMethod,
    pub confidence: Confidence,
    pub raw_string: Option<String>,
}

/// Confidence level for version detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for DartVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        match self.channel {
            Channel::Stable => write!(f, " (stable)")?,
            Channel::Beta => write!(f, " (beta)")?,
            Channel::Dev => write!(f, " (dev)")?,
            Channel::Unknown => {}
        }
        if self.confidence != Confidence::High {
            write!(f, " [confidence: {:?}]", self.confidence)?;
        }
        Ok(())
    }
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Channel::Stable => write!(f, "stable"),
            Channel::Beta => write!(f, "beta"),
            Channel::Dev => write!(f, "dev"),
            Channel::Unknown => write!(f, "unknown"),
        }
    }
}

/// Detect Dart VM version using multiple strategies
pub fn detect_dart_version(
    file: &BinaryFile,
    _parsed: &ParsedBinary,
) -> Result<DartVersion, CoreError> {
    // Strategy 1: Search for version string
    if let Some(version) = detect_by_version_string(file) {
        info!("Dart version detected via version string: {}", version);
        return Ok(version);
    }

    // Strategy 2: Search for SDK hash
    if let Some(version) = detect_by_sdk_hash(file) {
        info!("Dart version detected via SDK hash: {}", version);
        return Ok(version);
    }

    // Strategy 3: Fingerprint by struct sizes
    if let Some(version) = detect_by_fingerprint(file) {
        info!(
            "Dart version detected via fingerprinting: {} (confidence: {:?})",
            version, version.confidence
        );
        return Ok(version);
    }

    // Strategy 4: Stub comparison
    if let Some(version) = detect_by_stub_comparison(file) {
        info!(
            "Dart version detected via stub comparison: {} (confidence: {:?})",
            version, version.confidence
        );
        return Ok(version);
    }

    Err(CoreError::VersionNotDetected)
}

fn detect_by_version_string(file: &BinaryFile) -> Option<DartVersion> {
    let data = file.data();

    // Pattern: "X.Y.Z (stable)" or "X.Y.Z (dev)" or "X.Y.Z (beta)"
    let re = Regex::new(r"(\d+)\.(\d+)\.(\d+)\s*\((stable|beta|dev)\)").ok()?;

    // Search in .rodata-like regions — scan the whole file with memchr first
    // Look for common version prefix patterns
    let patterns = [b"Dart/" as &[u8], b"dart-sdk" as &[u8], b"(stable)" as &[u8], b"(beta)" as &[u8], b"(dev)" as &[u8], b"flutter" as &[u8]];

    for pattern in &patterns {
        for offset in memchr::memmem::find_iter(data, *pattern) {
            // Read a chunk around the match (look back too for version before marker)
            let start = offset.saturating_sub(32);
            let end = (offset + 256).min(data.len());
            if let Ok(text) = std::str::from_utf8(&data[start..end]) {
                if let Some(caps) = re.captures(text) {
                    let major: u32 = caps[1].parse().ok()?;
                    let minor: u32 = caps[2].parse().ok()?;
                    let patch: u32 = caps[3].parse().ok()?;
                    let channel = match &caps[4] {
                        "stable" => Channel::Stable,
                        "beta" => Channel::Beta,
                        "dev" => Channel::Dev,
                        _ => Channel::Unknown,
                    };

                    return Some(DartVersion {
                        major,
                        minor,
                        patch,
                        channel,
                        sdk_hash: None,
                        detection_method: DetectionMethod::VersionString,
                        confidence: Confidence::High,
                        raw_string: Some(caps[0].to_string()),
                    });
                }
            }
        }
    }

    // Also try a broader scan for version patterns without prefix
    let version_re = Regex::new(r"(\d+)\.(\d+)\.(\d+)\s*\((stable|beta|dev)\)").ok()?;
    // Scan through the file in chunks using lossy UTF-8 conversion
    let chunk_size = 4096;
    for chunk_start in (0..data.len()).step_by(chunk_size) {
        let chunk_end = (chunk_start + chunk_size + 64).min(data.len());
        let text = String::from_utf8_lossy(&data[chunk_start..chunk_end]);
        if let Some(caps) = version_re.captures(&text) {
                let major: u32 = caps[1].parse().ok()?;
                let minor: u32 = caps[2].parse().ok()?;
                let patch: u32 = caps[3].parse().ok()?;

                // Validate: Dart versions are typically 2.x or 3.x
                if major < 2 || major > 4 {
                    continue;
                }

                let channel = match &caps[4] {
                    "stable" => Channel::Stable,
                    "beta" => Channel::Beta,
                    "dev" => Channel::Dev,
                    _ => Channel::Unknown,
                };

                return Some(DartVersion {
                    major,
                    minor,
                    patch,
                    channel,
                    sdk_hash: None,
                    detection_method: DetectionMethod::VersionString,
                    confidence: Confidence::High,
                    raw_string: Some(caps[0].to_string()),
                });
            }
    }

    None
}

fn detect_by_sdk_hash(file: &BinaryFile) -> Option<DartVersion> {
    let data = file.data();

    // Look for "dart-sdk-hash:" followed by 40 hex chars
    let marker = b"dart-sdk-hash:";
    for offset in memchr::memmem::find_iter(data, marker) {
        let hash_start = offset + marker.len();
        let hash_end = hash_start + 40;
        if hash_end <= data.len() {
            if let Ok(hash_str) = std::str::from_utf8(&data[hash_start..hash_end]) {
                if hash_str.chars().all(|c| c.is_ascii_hexdigit()) {
                    debug!("Found SDK hash: {}", hash_str);

                    // Map known hashes to versions
                    if let Some(version) = lookup_sdk_hash(hash_str) {
                        return Some(version);
                    }

                    // Unknown hash — return with low confidence
                    return Some(DartVersion {
                        major: 3,
                        minor: 0,
                        patch: 0,
                        channel: Channel::Unknown,
                        sdk_hash: Some(hash_str.to_string()),
                        detection_method: DetectionMethod::SdkHash,
                        confidence: Confidence::Low,
                        raw_string: None,
                    });
                }
            }
        }
    }

    None
}

fn lookup_sdk_hash(_hash: &str) -> Option<DartVersion> {
    // Known SDK hashes — extend as needed
    // In production this would be a large lookup table
    None
}

fn detect_by_fingerprint(file: &BinaryFile) -> Option<DartVersion> {
    let data = file.data();

    // Snapshot magic bytes for different Dart versions
    let dart3_magic = [0xf5u8, 0xf5, 0xdc, 0xdc];
    let dart2_magic = [0xf5u8, 0xf5, 0xdc, 0xdc]; // Same magic, different internal layout

    if memchr::memmem::find(data, &dart3_magic).is_some() {
        // Found snapshot magic — likely Dart 2.x or 3.x
        // Distinguish by looking at snapshot features string

        // Look for "null-safety" feature flag (Dart 2.12+)
        let has_null_safety = memchr::memmem::find(data, b"null-safety").is_some();
        // Look for "records" feature (Dart 3.0+)
        let has_records = memchr::memmem::find(data, b"records").is_some();
        // Look for "sealed-classes" (Dart 3.0+)
        let has_sealed = memchr::memmem::find(data, b"sealed-class").is_some();

        if has_records || has_sealed {
            return Some(DartVersion {
                major: 3,
                minor: 0,
                patch: 0,
                channel: Channel::Unknown,
                sdk_hash: None,
                detection_method: DetectionMethod::StructFingerprint,
                confidence: Confidence::Medium,
                raw_string: None,
            });
        } else if has_null_safety {
            return Some(DartVersion {
                major: 2,
                minor: 19,
                patch: 0,
                channel: Channel::Unknown,
                sdk_hash: None,
                detection_method: DetectionMethod::StructFingerprint,
                confidence: Confidence::Medium,
                raw_string: None,
            });
        }
    }

    warn!("Could not fingerprint Dart version from struct sizes");
    None
}

fn detect_by_stub_comparison(_file: &BinaryFile) -> Option<DartVersion> {
    // Stub comparison requires known patterns for each version
    // This is a placeholder for the heuristic approach
    warn!("Stub comparison not yet implemented for version detection");
    None
}
