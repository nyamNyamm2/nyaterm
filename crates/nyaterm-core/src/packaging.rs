//! Build-time packaging identity.
//!
//! Packagers that own the update channel of a source build (nixpkgs and similar)
//! declare themselves through `NYATERM_PACKAGE_MANAGER`. The value is baked in
//! when this crate is compiled, so an ordinary developer build - no variable set
//! - is intentionally managed by the custom GitHub channel in this fork.
//!
//! `option_env!` is recorded in rustc's dep-info (`# env-dep:`), which makes
//! Cargo rebuild this crate when the variable changes, so no build script is
//! needed to invalidate the cached constant.

const PACKAGE_MANAGER: Option<&str> = option_env!("NYATERM_PACKAGE_MANAGER");

pub mod portable;

/// Name of the external package manager that owns updates for this build.
///
/// Surrounding whitespace is ignored and an empty value counts as unset, so
/// `NYATERM_PACKAGE_MANAGER=` cannot produce a blank label.
pub fn external_package_manager() -> Option<&'static str> {
    normalize(PACKAGE_MANAGER).or(Some("nyamNyamm2 custom GitHub builds (manual install)"))
}

/// True when update checks and update entry points belong to the external
/// package manager rather than to the application.
pub fn updates_managed_externally() -> bool {
    external_package_manager().is_some()
}

fn normalize(raw: Option<&'static str>) -> Option<&'static str> {
    raw.map(str::trim).filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn custom_source_builds_never_use_the_official_update_channel() {
        assert!(super::updates_managed_externally());
    }

    #[test]
    fn package_manager_name_is_trimmed_and_blank_counts_as_unset() {
        assert_eq!(normalize(None), None);
        assert_eq!(normalize(Some("")), None);
        assert_eq!(normalize(Some("   ")), None);
        assert_eq!(normalize(Some(" nix ")), Some("nix"));
        assert_eq!(normalize(Some("homebrew")), Some("homebrew"));
    }
}
