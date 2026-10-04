//! Embeds the version of the build into the binary (see `src/version.rs`):
//!
//! - `ULU_COMMIT_COUNT`: number of commits, used as monotonic build number
//! - `ULU_COMMIT_HASH`: short commit hash, with `-dirty` for uncommitted changes
//! - `ULU_BUILD_TIME`: time of the build in UTC
//!
//! The commit count and hash can be passed in as environment variables of the
//! same name, e.g. when building in a container without git (see `Makefile`).

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    for path in [
        ".git/HEAD",
        ".git/index",
        ".git/refs",
        "src",
        "assets",
        "Cargo.toml",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    for var in ["ULU_COMMIT_COUNT", "ULU_COMMIT_HASH"] {
        println!("cargo:rerun-if-env-changed={var}");
    }

    let count = env_or("ULU_COMMIT_COUNT", || git(&["rev-list", "--count", "HEAD"]));
    let hash = env_or("ULU_COMMIT_HASH", || {
        let hash = git(&["rev-parse", "--short", "HEAD"])?;
        let dirty = !git(&["status", "--porcelain", "--untracked-files=no"])?.is_empty();
        Some(if dirty { format!("{hash}-dirty") } else { hash })
    });
    println!(
        "cargo:rustc-env=ULU_COMMIT_COUNT={}",
        count.unwrap_or_else(|| "0".into())
    );
    println!(
        "cargo:rustc-env=ULU_COMMIT_HASH={}",
        hash.unwrap_or_else(|| "unknown".into())
    );
    println!("cargo:rustc-env=ULU_BUILD_TIME={}", build_time());
}

fn env_or(var: &str, fallback: impl FnOnce() -> Option<String>) -> Option<String> {
    std::env::var(var)
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(fallback)
}

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_string())
}

/// The current time in UTC, e.g. `2026-10-04 21:30 UTC`.
fn build_time() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() as i64);
    let (days, secs_of_day) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    let (hour, minute) = (secs_of_day / 3600, secs_of_day % 3600 / 60);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02} UTC")
}

/// Converts days since 1970-01-01 to a date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}
