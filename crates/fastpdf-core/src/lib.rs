//! FastPDF core — PDF parsing, rendering, and manipulation (no WASM).

/// Engine version string (semver aligned with workspace).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_set() {
        assert!(!VERSION.is_empty());
    }
}
