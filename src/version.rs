//! Version of the build, embedded by `build.rs`.

/// Monotonic version: the crate version followed by the number of commits,
/// e.g. `0.1.0.81`.
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), ".", env!("ULU_COMMIT_COUNT"));

/// Short hash of the commit, with `-dirty` if there were uncommitted changes.
pub const COMMIT: &str = env!("ULU_COMMIT_HASH");

/// When the binary was built, in UTC.
pub const BUILD_TIME: &str = env!("ULU_BUILD_TIME");

/// Everything in one line, e.g. `0.1.0.81 (a47f444, 2026-10-04 21:30 UTC)`.
pub fn long() -> String {
    format!("{VERSION} ({COMMIT}, {BUILD_TIME})")
}
