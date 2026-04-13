/// Android permissions analysis (from AndroidManifest.xml)
pub fn analyze_permissions(manifest_content: &str) -> Vec<String> {
    let mut permissions = Vec::new();
    for line in manifest_content.lines() {
        if line.contains("uses-permission") {
            if let Some(start) = line.find("android.permission.") {
                let perm = &line[start..];
                if let Some(end) = perm.find('"') {
                    permissions.push(perm[..end].to_string());
                }
            }
        }
    }
    permissions
}
