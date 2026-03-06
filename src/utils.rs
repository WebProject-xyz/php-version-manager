use semver::Version;

/// Sorts a list of version strings using semantic versioning.
/// If a version string is not valid semver, it falls back to a simple string-based numeric sort.
pub fn sort_versions(versions: &mut [String]) {
    versions.sort_by(|a, b| {
        let a_sem = Version::parse(a);
        let b_sem = Version::parse(b);

        match (a_sem, b_sem) {
            (Ok(av), Ok(bv)) => av.cmp(&bv),
            _ => {
                // Fallback for non-semver strings (e.g., "8.2")
                let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
                let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
                a_parts.cmp(&b_parts)
            }
        }
    });
}
