use semver::Version;

/// Sorts a list of version strings using semantic versioning.
/// If a version string is not valid semver, it falls back to a simple string-based numeric sort.
pub fn sort_versions(versions: &mut [String]) {
    versions.sort_by(|a, b| {
        let a_sem = Version::parse(a);
        let b_sem = Version::parse(b);

        match (a_sem, b_sem) {
            (Ok(av), Ok(bv)) => av.cmp(&bv),
            (Ok(av), Err(_)) => {
                let b_parts: Vec<u64> = b.split('.').filter_map(|s| s.parse().ok()).collect();
                let a_parts = vec![av.major, av.minor, av.patch];
                match a_parts.cmp(&b_parts) {
                    std::cmp::Ordering::Equal => {
                        if !av.pre.is_empty() {
                            std::cmp::Ordering::Less
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    }
                    ord => ord,
                }
            }
            (Err(_), Ok(bv)) => {
                let a_parts: Vec<u64> = a.split('.').filter_map(|s| s.parse().ok()).collect();
                let b_parts = vec![bv.major, bv.minor, bv.patch];
                match a_parts.cmp(&b_parts) {
                    std::cmp::Ordering::Equal => {
                        if !bv.pre.is_empty() {
                            std::cmp::Ordering::Greater
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    }
                    ord => ord,
                }
            }
            _ => {
                // Fallback for non-semver strings (e.g., "8.2")
                let a_parts: Vec<u64> = a.split('.').filter_map(|s| s.parse().ok()).collect();
                let b_parts: Vec<u64> = b.split('.').filter_map(|s| s.parse().ok()).collect();
                a_parts.cmp(&b_parts)
            }
        }
    });
}
