//! The cink Typst package as embedded data, for Rust hosts that run
//! Typst in-process.
//!
//! The crate ships the same `typst.toml`, `typst/` and `tables/` files
//! that make up the Typst package, so a host can register them with its
//! Typst world instead of downloading or vendoring a copy. Rust and Typst
//! then always come from the same release.
//!
//! The Typst sources read their tables at runtime, so `tables/` belongs to
//! the package as well.
//!
//! ```
//! let spec = cink::typst::spec();
//! assert_eq!(spec, format!("@preview/cink:{}", cink::typst::VERSION));
//! for (path, bytes) in cink::typst::FILES {
//!     // Register `bytes` at package-relative `path` for `spec`.
//!     assert!(!path.starts_with('/') && !bytes.is_empty());
//! }
//! ```

/// Package namespace used in Typst imports (`@preview/...`).
pub const NAMESPACE: &str = "preview";

/// Package name, as declared in `typst.toml`.
pub const NAME: &str = "cink";

/// Package version; the Typst package follows the crate release line.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Package files as `(package-relative path, contents)`, sorted by path.
/// Paths use `/` separators; `typst.toml` names the entrypoint.
pub const FILES: &[(&str, &[u8])] = &[
    ("tables/README.md", include_bytes!("../tables/README.md")),
    (
        "tables/defaults.json",
        include_bytes!("../tables/defaults.json"),
    ),
    ("typst.toml", include_bytes!("../typst.toml")),
    ("typst/ink.typ", include_bytes!("../typst/ink.typ")),
];

/// Package specification for Typst imports: `@preview/cink:<version>`.
#[must_use]
pub fn spec() -> String {
    format!("@{NAMESPACE}/{NAME}:{VERSION}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).expect("readable directory") {
            let path = entry.expect("readable entry").path();
            if path.is_dir() {
                collect(root, &path, out);
            } else {
                let relative: Vec<String> = path
                    .strip_prefix(root)
                    .expect("path below the crate root")
                    .components()
                    .map(|part| part.as_os_str().to_str().expect("UTF-8 path").to_owned())
                    .collect();
                out.push((
                    relative.join("/"),
                    std::fs::read(&path).expect("readable file"),
                ));
            }
        }
    }

    /// Value of `key = "..."` in the `[package]` table of `typst.toml`.
    fn manifest_value(key: &str) -> String {
        let (_, manifest) = FILES
            .iter()
            .find(|(path, _)| *path == "typst.toml")
            .expect("typst.toml is embedded");
        let manifest = std::str::from_utf8(manifest).expect("typst.toml is UTF-8");
        let mut in_package = false;
        for line in manifest.lines().map(str::trim) {
            if line.starts_with('[') {
                in_package = line == "[package]";
            } else if in_package
                && let Some((name, value)) = line.split_once('=')
                && name.trim() == key
            {
                return value
                    .trim()
                    .strip_prefix('"')
                    .and_then(|rest| rest.strip_suffix('"'))
                    .expect("quoted string value")
                    .to_owned();
            }
        }
        panic!("typst.toml has no [package] {key}");
    }

    #[test]
    fn files_match_disk() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut disk = vec![(
            "typst.toml".to_owned(),
            std::fs::read(root.join("typst.toml")).expect("typst.toml exists"),
        )];
        collect(root, &root.join("typst"), &mut disk);
        collect(root, &root.join("tables"), &mut disk);
        disk.sort();
        let embedded: Vec<(String, Vec<u8>)> = FILES
            .iter()
            .map(|(path, bytes)| ((*path).to_owned(), bytes.to_vec()))
            .collect();
        let paths = |files: &[(String, Vec<u8>)]| -> Vec<String> {
            files.iter().map(|(path, _)| path.clone()).collect()
        };
        assert_eq!(
            paths(&embedded),
            paths(&disk),
            "FILES lists exactly the package files, sorted"
        );
        for ((path, embedded), (_, disk)) in embedded.iter().zip(&disk) {
            assert!(embedded == disk, "{path}: embedded bytes differ from disk");
        }
    }

    #[test]
    fn manifest_matches_crate() {
        assert_eq!(manifest_value("name"), NAME);
        assert_eq!(manifest_value("version"), VERSION);
        assert_eq!(NAME, env!("CARGO_PKG_NAME"));
        let entrypoint = manifest_value("entrypoint");
        assert!(
            FILES.iter().any(|(path, _)| *path == entrypoint),
            "entrypoint {entrypoint:?} is embedded"
        );
        assert_eq!(
            spec(),
            format!("@preview/cink:{}", env!("CARGO_PKG_VERSION"))
        );
    }
}
