use crate::schema::DartProfile;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Embedded profiles compiled into the binary
static EMBEDDED_PROFILES: Lazy<HashMap<String, DartProfile>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Embed known profiles at compile time
    let profiles_data: &[(&str, &str)] = &[
        ("2.19.0", include_str!("../profiles/dart_2.19.json")),
        ("3.0.0", include_str!("../profiles/dart_3.0.json")),
        ("3.2.0", include_str!("../profiles/dart_3.2.json")),
        ("3.5.0", include_str!("../profiles/dart_3.5.json")),
    ];

    for (version, data) in profiles_data {
        match serde_json::from_str::<DartProfile>(data) {
            Ok(profile) => {
                map.insert(version.to_string(), profile);
            }
            Err(e) => {
                eprintln!("Failed to parse embedded profile {}: {}", version, e);
            }
        }
    }

    map
});

/// Profile resolver that loads and caches Dart VM profiles
pub struct ProfileResolver {
    profiles: HashMap<String, DartProfile>,
    search_paths: Vec<PathBuf>,
}

impl ProfileResolver {
    /// Create a new resolver with embedded profiles
    pub fn new() -> Self {
        Self {
            profiles: EMBEDDED_PROFILES.clone(),
            search_paths: vec![],
        }
    }

    /// Add a directory to search for profile JSON files
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Load all profiles from search paths
    pub fn load_external_profiles(&mut self) {
        for search_path in &self.search_paths.clone() {
            if let Ok(entries) = std::fs::read_dir(search_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "json").unwrap_or(false) {
                        if let Ok(data) = std::fs::read_to_string(&path) {
                            match serde_json::from_str::<DartProfile>(&data) {
                                Ok(profile) => {
                                    info!("Loaded profile: {} from {:?}", profile.version, path);
                                    self.profiles
                                        .insert(profile.version.clone(), profile);
                                }
                                Err(e) => {
                                    warn!("Failed to parse profile {:?}: {}", path, e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Load a single profile from a file
    pub fn load_profile_file(&mut self, path: &Path) -> Result<(), String> {
        let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let profile: DartProfile = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        self.profiles.insert(profile.version.clone(), profile);
        Ok(())
    }

    /// Resolve a profile for a given version string (with fuzzy matching)
    pub fn resolve(&self, major: u32, minor: u32, patch: u32) -> Option<&DartProfile> {
        let exact_key = format!("{}.{}.{}", major, minor, patch);

        // 1. Exact match
        if let Some(profile) = self.profiles.get(&exact_key) {
            debug!("Exact profile match: {}", exact_key);
            return Some(profile);
        }

        // 2. Same major.minor, closest patch (patch differences usually don't change layout)
        let minor_key = format!("{}.{}", major, minor);
        let mut best: Option<(&str, &DartProfile, i32)> = None;

        for (key, profile) in &self.profiles {
            if key.starts_with(&minor_key) {
                let parts: Vec<&str> = key.split('.').collect();
                if parts.len() >= 3 {
                    if let Ok(p) = parts[2].parse::<u32>() {
                        let diff = (p as i32 - patch as i32).abs();
                        if best.is_none() || diff < best.unwrap().2 {
                            best = Some((key, profile, diff));
                        }
                    }
                }
            }
        }

        if let Some((key, profile, _)) = best {
            warn!(
                "No exact profile for {}; using closest match: {}",
                exact_key, key
            );
            return Some(profile);
        }

        // 3. Same major version, closest minor
        let major_prefix = format!("{}.", major);
        let mut best_major: Option<(&str, &DartProfile, i32)> = None;

        for (key, profile) in &self.profiles {
            if key.starts_with(&major_prefix) {
                let parts: Vec<&str> = key.split('.').collect();
                if parts.len() >= 2 {
                    if let Ok(m) = parts[1].parse::<u32>() {
                        let diff = (m as i32 - minor as i32).abs();
                        if best_major.is_none() || diff < best_major.unwrap().2 {
                            best_major = Some((key, profile, diff));
                        }
                    }
                }
            }
        }

        if let Some((key, profile, _)) = best_major {
            warn!(
                "No minor version match for {}; using nearest: {} (WARNING: may be inaccurate)",
                exact_key, key
            );
            return Some(profile);
        }

        warn!(
            "No profile found for version {}. Run: dart_dec profile-gen --tag {}.{}.{}",
            exact_key, major, minor, patch
        );
        None
    }

    /// Get all available profile version strings
    pub fn available_versions(&self) -> Vec<&str> {
        let mut versions: Vec<&str> = self.profiles.keys().map(|s| s.as_str()).collect();
        versions.sort();
        versions
    }
}

impl Default for ProfileResolver {
    fn default() -> Self {
        Self::new()
    }
}
