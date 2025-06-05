use std::path::{Path, PathBuf};

// Finds the shortest relative path to a header file from a list of include paths
pub(crate) fn header_path(header: &Path, include_paths: &[PathBuf]) -> String {
    let canonic_header = canonicalize(header);

    let mut maybe_best_match: Option<PathBuf> = None;
    for include_path in include_paths {
        let include_path = canonicalize(include_path);
        let relative = pathdiff::diff_paths(&canonic_header, include_path);
        if let Some(relative) = relative {
            if let Some(best_match) = maybe_best_match.as_ref() {
                if relative.components().count() < best_match.components().count() {
                    maybe_best_match = Some(relative)
                }
            } else {
                maybe_best_match = Some(relative)
            }
        }
    }

    maybe_best_match
        .as_deref()
        .unwrap_or(header)
        .to_str()
        .expect("Path should be valid UTF-8")
        .replace('\\', "/")
}

fn canonicalize(path: &Path) -> PathBuf {
    // Use dunce to avoid "verbatim disk" style in Windows if the path exists
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_path_under_include_paths() {
        let include_paths = vec![
            PathBuf::from("/usr/include"),
            PathBuf::from("/usr/local/include"),
        ];

        let result = header_path(&PathBuf::from("/usr/include/header.h"), &include_paths);
        assert_eq!(result, "header.h");

        let result = header_path(
            &PathBuf::from("/usr/local/include/another/header.h"),
            &include_paths,
        );
        assert_eq!(result, "another/header.h");
    }

    #[test]
    fn test_header_path_outside_include_paths() {
        let include_paths = vec![
            PathBuf::from("/usr/include"),
            PathBuf::from("/usr/local/include"),
        ];

        let result = header_path(
            &PathBuf::from("/home/user/project/include/header.h"),
            &include_paths,
        );
        assert_eq!(result, "../../home/user/project/include/header.h");

        let result = header_path(&PathBuf::from("/usr/local/header.h"), &include_paths);
        assert_eq!(result, "../header.h");
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_style_paths() {
        let include_paths = vec![
            PathBuf::from(r"C:\Windows"),
            PathBuf::from(r"C:\Unknown"),
            PathBuf::from(r"C:\temp"),
        ];

        let result = header_path(&PathBuf::from(r"C:\Windows\header.h"), &include_paths);
        assert_eq!(result, "header.h");

        let result = header_path(
            &PathBuf::from(r"C:\Windows\include\header.h"),
            &include_paths,
        );
        assert_eq!(result, "include/header.h");

        let result = header_path(&PathBuf::from(r"C:\temp\header.h"), &include_paths);
        assert_eq!(result, "header.h");
    }

    #[test]
    #[cfg(windows)]
    fn canonicalize_avoids_verbatim_disk_on_windows() {
        let path = PathBuf::from(r"C:\DoesNotExist");
        assert_eq!(Some(r"C:\DoesNotExist"), canonicalize(&path).to_str());
        // This path most likely exists on a Windows system. When a path exists,
        // unfortunately Rust standard canconicalize() normally replaces the disk letter
        // with a "verbatim disk", e.g., "C:\\" becomes "\\?\C:\".
        let path = PathBuf::from(r"C:\Windows");
        assert_eq!(Some(r"C:\Windows"), canonicalize(&path).to_str());
    }
}
