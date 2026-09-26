//! File-based language progression checks.
//! Run with `cargo test -p shardscli --test language_levels -- --nocapture` for the report.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const LEVELS: std::ops::RangeInclusive<u8> = 1..=10;

fn fixture(language: &str, level: u8, extension: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(language)
        .join(format!("{level:02}.{extension}"))
}

fn expected_stdout(language: &str, level: u8) -> Option<&'static str> {
    match (language, level) {
        ("sh", 1) => Some("sh-01\n"),
        ("sh", 2) => Some("sh level 02\n"),
        ("sh", 3) => Some("sh-03\n"),
        ("sh", 4) => Some("sh-04\n"),
        ("sh", 5) => Some("sh-05\n"),
        ("sh", 6) => Some("sh-06"),
        ("sh", 7) => Some("sh-07"),
        ("sh", 8) => Some("sh-08\n"),
        ("sh", 9) => Some("sh-09\n"),
        ("sh", 10) => Some("sh-10\n"),
        ("rust", 1) => Some("rust-01\n"),
        ("rust", 2) => Some("rust level 02\n"),
        ("rust", 3) => Some("rust 03\n"),
        ("rust", 4) => Some("rust-04\n"),
        ("rust", 5) => Some("rust-05\n"),
        ("rust", 6) => Some("rust-06\n"),
        ("rust", 7) => Some("rust-07\n"),
        ("rust", 8) => Some("rust-08\n"),
        ("rust", 9) => Some("rust-09\n"),
        ("rust", 10) => Some("rust-10\n"),
        ("julia", 1) => Some("julia-01\n"),
        ("julia", 2) => Some("julia level 02\n"),
        ("julia", 3) => Some("julia-03\n"),
        ("julia", 4) => Some("julia-04\n"),
        ("julia", 5) => Some("julia-05\n"),
        ("julia", 6) => Some("julia-06\n"),
        ("julia", 7) => Some("julia-07\n"),
        ("julia", 8) => Some("julia-08\n"),
        ("julia", 9) => Some("julia-09\n"),
        ("julia", 10) => Some("julia-10\n"),
        _ => None,
    }
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(language: &str, level: u8) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "shards-levels-{}-{language}-{level:02}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn execute(language: &str, level: u8, extension: &str) -> Output {
    let scratch = Scratch::new(language, level);
    Command::new(env!("CARGO_BIN_EXE_shards"))
        .arg("--lang")
        .arg(language)
        .arg(fixture(language, level, extension))
        .current_dir(&scratch.0)
        .output()
        .unwrap()
}

#[test]
fn every_language_has_ten_source_files() {
    for (language, extension) in [("sh", "sh"), ("rust", "rsh"), ("julia", "jl")] {
        let dir = fixture(language, 1, extension).parent().unwrap().to_owned();
        let count = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|value| value == extension))
            .count();
        assert_eq!(count, 10, "{language} fixture count");
        for level in LEVELS {
            assert!(
                fixture(language, level, extension).is_file(),
                "{language} {level:02}"
            );
        }
    }
}

#[test]
fn report_file_execution_levels() {
    for (language, extension, required_through) in
        [("sh", "sh", 8), ("rust", "rsh", 6), ("julia", "jl", 6)]
    {
        for level in LEVELS {
            let output = execute(language, level, extension);
            let expected = expected_stdout(language, level).unwrap();
            let passed = output.status.success() && output.stdout == expected.as_bytes();
            eprintln!(
                "{language} {level:02}: {}{}",
                if passed { "PASS" } else { "FAIL" },
                if passed {
                    String::new()
                } else {
                    format!(
                        " (status={:?}, stdout={:?}, stderr={:?})",
                        output.status.code(),
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    )
                }
            );
            if level <= required_through {
                assert!(
                    passed,
                    "{language} level {level:02} must execute successfully"
                );
            }
        }
    }
}
