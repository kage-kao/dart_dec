#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_profile_resolver_new() {
        let resolver = ProfileResolver::new();
        let versions = resolver.available_versions();
        assert!(versions.len() >= 4);
        assert!(versions.contains(&"2.19.0"));
        assert!(versions.contains(&"3.0.0"));
        assert!(versions.contains(&"3.2.0"));
        assert!(versions.contains(&"3.5.0"));
    }

    #[test]
    fn test_exact_version_resolve() {
        let resolver = ProfileResolver::new();
        let profile = resolver.resolve(3, 2, 0);
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().version, "3.2.0");
    }

    #[test]
    fn test_fuzzy_version_resolve_patch() {
        let resolver = ProfileResolver::new();
        // 3.2.3 should resolve to 3.2.0 (closest patch)
        let profile = resolver.resolve(3, 2, 3);
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().version, "3.2.0");
    }

    #[test]
    fn test_fuzzy_version_resolve_minor() {
        let resolver = ProfileResolver::new();
        // 3.1.0 should resolve to nearest 3.x
        let profile = resolver.resolve(3, 1, 0);
        assert!(profile.is_some());
    }

    #[test]
    fn test_profile_class_ids() {
        let resolver = ProfileResolver::new();
        let profile = resolver.resolve(3, 2, 0).unwrap();
        assert_eq!(profile.class_id("OneByteString"), Some(78));
        assert_eq!(profile.class_id("Null"), Some(66));
        assert_eq!(profile.class_id("Bool"), Some(65));
        assert_eq!(profile.class_id("Record"), Some(80));
    }

    #[test]
    fn test_profile_arch_config() {
        let resolver = ProfileResolver::new();
        let profile = resolver.resolve(3, 2, 0).unwrap();

        let arm64_cfg = profile.arch_config("arm64").unwrap();
        assert!(arm64_cfg.compressed_pointers);
        assert_eq!(arm64_cfg.pointer_size, 4);
        assert_eq!(arm64_cfg.object_alignment, 8);

        let x86_cfg = profile.arch_config("x86_64").unwrap();
        assert!(!x86_cfg.compressed_pointers);
        assert_eq!(x86_cfg.pointer_size, 8);
    }

    #[test]
    fn test_profile_snapshot_header() {
        let resolver = ProfileResolver::new();
        let profile = resolver.resolve(3, 2, 0).unwrap();
        assert_eq!(profile.snapshot_header.magic_value, "0xf5f5dcdc");
        assert_eq!(profile.snapshot_header.base_objects_offset, 64);
    }

    #[test]
    fn test_object_tags_parsing() {
        let resolver = ProfileResolver::new();
        let profile = resolver.resolve(3, 2, 0).unwrap();
        assert_eq!(profile.object_tags.class_id_mask_value(), 0xFFFF);
        assert_eq!(profile.object_tags.class_id_shift, 16);
    }

    #[test]
    fn test_class_layout() {
        let resolver = ProfileResolver::new();
        let profile = resolver.resolve(3, 2, 0).unwrap();
        let raw_class = profile.class_layout_by_name("RawClass");
        assert!(raw_class.is_some());
    }

    #[test]
    fn test_pointer_size() {
        let resolver = ProfileResolver::new();
        let profile = resolver.resolve(3, 2, 0).unwrap();
        assert_eq!(profile.pointer_size("arm64"), 4);
        assert_eq!(profile.pointer_size("x86_64"), 8);
        assert_eq!(profile.pointer_size("unknown"), 8); // default
    }

    #[test]
    fn test_compressed_pointers() {
        let resolver = ProfileResolver::new();
        let profile = resolver.resolve(3, 2, 0).unwrap();
        assert!(profile.uses_compressed_pointers("arm64"));
        assert!(!profile.uses_compressed_pointers("x86_64"));
    }
}
