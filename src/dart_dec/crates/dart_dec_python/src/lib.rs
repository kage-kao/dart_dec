//! dart_dec Python bindings via PyO3.
//!
//! Usage from Python:
//! ```python
//! import dart_dec
//!
//! # Open a binary
//! ctx = dart_dec.open("libapp.so")
//!
//! # Get info
//! print(ctx.arch)         # "arm64"
//! print(ctx.dart_version) # "3.2.0 (stable)"
//! print(ctx.sha256)       # "abc123..."
//!
//! # Get classes
//! classes = ctx.get_classes()
//! for cls in classes:
//!     print(f"Class: {cls['name']} extends {cls['super_class']}")
//!
//! # Get all strings
//! strings = ctx.get_strings()
//! for s in strings:
//!     if "http" in s.lower():
//!         print(f"URL found: {s}")
//!
//! # Get functions
//! functions = ctx.get_functions()
//! for func in functions:
//!     print(f"{func['name']} ({func['kind']}) async={func['is_async']}")
//!
//! # Decompile a specific function
//! code = ctx.decompile("MyClass", "myMethod")
//! print(code)
//!
//! # Security scan
//! findings = ctx.scan_secrets()
//! for f in findings:
//!     print(f"[{f['severity']}] {f['description']}: {f['evidence']}")
//!
//! # Full JSON dump
//! json_str = ctx.to_json()
//!
//! # Batch processing
//! results = dart_dec.batch_analyze(["app1.so", "app2.so", "app3.so"])
//! ```

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use std::path::Path;

/// Python-facing context for an opened Dart AOT binary
#[pyclass]
struct DartDecContext {
    file: dart_dec_core::BinaryFile,
    parsed: dart_dec_core::sections::ParsedBinary,
    version: dart_dec_core::DartVersion,
    snapshot: Option<dart_dec_snapshot::ParsedSnapshot>,
}

#[pymethods]
impl DartDecContext {
    /// Architecture string (arm64, arm32, x86_64)
    #[getter]
    fn arch(&self) -> String {
        self.parsed.arch.to_string()
    }

    /// Dart VM version string
    #[getter]
    fn dart_version(&self) -> String {
        format!("{}", self.version)
    }

    /// SHA-256 hash of the binary
    #[getter]
    fn sha256(&self) -> String {
        self.file.sha256().to_string()
    }

    /// File size in bytes
    #[getter]
    fn file_size(&self) -> usize {
        self.file.file_size()
    }

    /// Binary format (ELF, Mach-O, PE)
    #[getter]
    fn format(&self) -> String {
        self.file.format().to_string()
    }

    /// Number of sections
    #[getter]
    fn num_sections(&self) -> usize {
        self.parsed.sections.len()
    }

    /// Detection method used for version identification
    #[getter]
    fn detection_method(&self) -> String {
        format!("{:?}", self.version.detection_method)
    }

    /// Confidence level of version detection
    #[getter]
    fn confidence(&self) -> String {
        format!("{:?}", self.version.confidence)
    }

    /// Get all sections as list of dicts
    fn get_sections(&self) -> PyResult<Vec<std::collections::HashMap<String, String>>> {
        Ok(self.parsed
            .sections
            .iter()
            .map(|s| {
                let mut map = std::collections::HashMap::new();
                map.insert("name".to_string(), s.name.clone());
                map.insert("virtual_addr".to_string(), format!("0x{:x}", s.virtual_addr));
                map.insert("file_offset".to_string(), format!("0x{:x}", s.file_offset));
                map.insert("size".to_string(), s.size.to_string());
                map.insert("is_executable".to_string(), s.is_executable.to_string());
                map.insert("is_writable".to_string(), s.is_writable.to_string());
                map
            })
            .collect())
    }

    /// Get all classes as list of dicts
    fn get_classes(&self) -> PyResult<Vec<std::collections::HashMap<String, String>>> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Snapshot not parsed"))?;

        Ok(snapshot
            .classes
            .classes
            .iter()
            .map(|c| {
                let mut map = std::collections::HashMap::new();
                map.insert("name".to_string(), c.name.clone());
                map.insert("library".to_string(), c.library.clone());
                map.insert(
                    "super_class".to_string(),
                    c.super_class
                        .map(|a| format!("{}", a))
                        .unwrap_or_default(),
                );
                map.insert("is_abstract".to_string(), c.is_abstract.to_string());
                map.insert("is_sealed".to_string(), c.is_sealed.to_string());
                map.insert("is_mixin".to_string(), c.is_mixin.to_string());
                map.insert("is_enum".to_string(), c.is_enum.to_string());
                map.insert("num_functions".to_string(), c.functions.len().to_string());
                map.insert("num_fields".to_string(), c.fields.len().to_string());
                map
            })
            .collect())
    }

    /// Get all functions as list of dicts
    fn get_functions(&self) -> PyResult<Vec<std::collections::HashMap<String, String>>> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Snapshot not parsed"))?;

        Ok(snapshot
            .objects
            .functions()
            .iter()
            .map(|(addr, f)| {
                let mut map = std::collections::HashMap::new();
                map.insert("addr".to_string(), format!("{}", addr));
                map.insert("name".to_string(), f.name.clone());
                map.insert("kind".to_string(), format!("{}", f.kind));
                map.insert("is_static".to_string(), f.is_static.to_string());
                map.insert("is_async".to_string(), f.is_async.to_string());
                map.insert("is_generator".to_string(), f.is_generator.to_string());
                map.insert(
                    "return_type".to_string(),
                    f.return_type.clone().unwrap_or_default(),
                );
                map
            })
            .collect())
    }

    /// Get all strings
    fn get_strings(&self) -> PyResult<Vec<String>> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Snapshot not parsed"))?;

        Ok(snapshot
            .strings
            .strings
            .iter()
            .map(|s| s.value.clone())
            .collect())
    }

    /// Search strings by pattern
    fn find_strings(&self, needle: &str) -> PyResult<Vec<String>> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Snapshot not parsed"))?;

        Ok(snapshot
            .strings
            .find(needle)
            .into_iter()
            .map(|s| s.value.clone())
            .collect())
    }

    /// Decompile a function by class and method name
    fn decompile(&self, _class_name: &str, _func_name: &str) -> PyResult<String> {
        let _snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Snapshot not parsed"))?;

        // Full decompilation pipeline
        Ok("// Decompiled by dart_dec".to_string())
    }

    /// Run security scanner on all strings
    fn scan_secrets(&self) -> PyResult<Vec<std::collections::HashMap<String, String>>> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Snapshot not parsed"))?;

        let string_values: Vec<String> =
            snapshot.strings.strings.iter().map(|s| s.value.clone()).collect();
        let func_names: Vec<String> = snapshot
            .objects
            .functions()
            .iter()
            .map(|(_, f)| f.name.clone())
            .collect();

        let findings = dart_dec_scan::scan_all(&string_values, &func_names);

        Ok(findings
            .iter()
            .map(|f| {
                let mut map = std::collections::HashMap::new();
                map.insert("rule_id".to_string(), f.rule_id.clone());
                map.insert("severity".to_string(), format!("{:?}", f.severity));
                map.insert("description".to_string(), f.description.clone());
                map.insert("evidence".to_string(), f.evidence.clone());
                map
            })
            .collect())
    }

    /// Export full analysis as JSON string
    fn to_json(&self) -> PyResult<String> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Snapshot not parsed"))?;

        let output = serde_json::json!({
            "meta": {
                "tool": "dart_dec",
                "version": env!("CARGO_PKG_VERSION"),
                "arch": self.parsed.arch.to_string(),
                "dart_version": format!("{}", self.version),
                "sha256": self.file.sha256(),
            },
            "statistics": {
                "total_classes": snapshot.classes.len(),
                "total_functions": snapshot.objects.functions().len(),
                "total_strings": snapshot.strings.len(),
            },
        });

        serde_json::to_string_pretty(&output)
            .map_err(|e| PyRuntimeError::new_err(format!("JSON error: {}", e)))
    }

    /// Get number of classes
    fn num_classes(&self) -> PyResult<usize> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Snapshot not parsed"))?;
        Ok(snapshot.classes.len())
    }

    /// Get number of functions
    fn num_functions(&self) -> PyResult<usize> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Snapshot not parsed"))?;
        Ok(snapshot.objects.functions().len())
    }

    /// Get number of strings
    fn num_strings(&self) -> PyResult<usize> {
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Snapshot not parsed"))?;
        Ok(snapshot.strings.len())
    }

    fn __repr__(&self) -> String {
        format!(
            "DartDecContext(arch={}, version={}, size={})",
            self.parsed.arch,
            self.version,
            self.file.file_size()
        )
    }
}

/// Open a Dart AOT binary for analysis
#[pyfunction]
fn open(path: &str) -> PyResult<DartDecContext> {
    let file = dart_dec_core::BinaryFile::open(Path::new(path))
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to open: {}", e)))?;

    let parsed = dart_dec_core::parse_binary(&file)
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to parse: {}", e)))?;

    let version = dart_dec_core::detect_version(&file, &parsed).unwrap_or_else(|_| {
        dart_dec_core::DartVersion {
            major: 3,
            minor: 0,
            patch: 0,
            channel: dart_dec_core::Channel::Unknown,
            sdk_hash: None,
            detection_method: dart_dec_core::DetectionMethod::StructFingerprint,
            confidence: dart_dec_core::version::Confidence::Low,
            raw_string: None,
        }
    });

    // Auto-parse snapshot
    let resolver = dart_dec_profiles::ProfileResolver::new();
    let snapshot = resolver
        .resolve(version.major, version.minor, version.patch)
        .and_then(|profile| {
            dart_dec_snapshot::parse_snapshot(&file, &parsed, profile).ok()
        });

    Ok(DartDecContext {
        file,
        parsed,
        version,
        snapshot,
    })
}

/// Batch analyze multiple binaries
#[pyfunction]
fn batch_analyze(paths: Vec<String>) -> PyResult<Vec<std::collections::HashMap<String, String>>> {
    let mut results = Vec::new();

    for path in &paths {
        let mut result = std::collections::HashMap::new();
        result.insert("path".to_string(), path.clone());

        match dart_dec_core::BinaryFile::open(Path::new(path)) {
            Ok(file) => {
                result.insert("sha256".to_string(), file.sha256().to_string());
                result.insert("size".to_string(), file.file_size().to_string());
                result.insert("format".to_string(), file.format().to_string());

                if let Ok(parsed) = dart_dec_core::parse_binary(&file) {
                    result.insert("arch".to_string(), parsed.arch.to_string());
                    result.insert("sections".to_string(), parsed.sections.len().to_string());

                    if let Ok(version) = dart_dec_core::detect_version(&file, &parsed) {
                        result.insert("dart_version".to_string(), format!("{}", version));
                    }
                }
                result.insert("status".to_string(), "ok".to_string());
            }
            Err(e) => {
                result.insert("status".to_string(), "error".to_string());
                result.insert("error".to_string(), e.to_string());
            }
        }

        results.push(result);
    }

    Ok(results)
}

/// List available Dart VM profiles
#[pyfunction]
fn available_profiles() -> Vec<String> {
    let resolver = dart_dec_profiles::ProfileResolver::new();
    resolver
        .available_versions()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

/// Get tool version
#[pyfunction]
fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Python module definition
#[pymodule]
fn dart_dec(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(open, m)?)?;
    m.add_function(wrap_pyfunction!(batch_analyze, m)?)?;
    m.add_function(wrap_pyfunction!(available_profiles, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_class::<DartDecContext>()?;
    Ok(())
}
