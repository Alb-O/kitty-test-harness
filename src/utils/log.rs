//! Test logging utilities for debugging terminal applications.
//!
//! Provides file-based logging that allows the application under test
//! to write debug messages that tests can read back. This is essential
//! for debugging issues where the application runs inside a kitty window
//! and its stderr is not visible to the test runner.
//!
//! # Usage
//!
//! 1. Create a test log file with [`create_test_log`]
//! 2. Pass the path to your application via environment variable
//! 3. Have your application write debug messages to this file
//! 4. Read back the log with [`read_test_log`] or wait for specific
//!    patterns with [`wait_for_log_line`]
//!
//! # Example
//!
//! ```no_run
//! use kitty_test_harness::utils::log::{create_test_log, read_test_log, cleanup_test_log};
//!
//! let log_path = create_test_log();
//!
//! // Pass to your application via env var, e.g.:
//! // TOME_TEST_LOG=/tmp/kitty-test-123-0.log ./my-app
//!
//! // Later, read the log:
//! let lines = read_test_log(&log_path);
//! for line in &lines {
//!     eprintln!("DEBUG: {}", line);
//! }
//!
//! cleanup_test_log(&log_path);
//! ```

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

static LOG_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Creates a unique test log file and returns its path.
///
/// The file is created in the system temp directory with a unique name
/// based on PID and counter to avoid collisions between parallel tests.
///
/// The file is created empty and ready for the application to append to.
pub fn create_test_log() -> PathBuf {
    let pid = std::process::id();
    let idx = LOG_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("kitty-test-{pid}-{idx}.log"));

    // Remove any existing file from a previous run
    let _ = fs::remove_file(&path);

    // Create empty file
    File::create(&path).expect("create test log file");
    path
}

/// Reads all lines from a test log file.
///
/// Returns an empty vector if the file doesn't exist or can't be read.
pub fn read_test_log(path: &Path) -> Vec<String> {
    if !path.exists() {
        return Vec::new();
    }
    let Ok(file) = File::open(path) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .map(|l| l.unwrap_or_default())
        .collect()
}

/// Waits for a log file to contain a line matching the predicate.
///
/// Polls the file every 10ms until timeout is reached.
/// Returns the first matching line, or `None` if timeout expires.
pub fn wait_for_log_line(
    path: &Path,
    timeout: Duration,
    predicate: impl Fn(&str) -> bool,
) -> Option<String> {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        for line in read_test_log(path) {
            if predicate(&line) {
                return Some(line);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

/// Removes a test log file.
///
/// Silently ignores errors (e.g., if file doesn't exist).
pub fn cleanup_test_log(path: &Path) {
    let _ = fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_create_and_read_log() {
        let path = create_test_log();
        assert!(path.exists());

        // Write some lines
        {
            let mut file = fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .expect("open for append");
            writeln!(file, "line 1").unwrap();
            writeln!(file, "line 2").unwrap();
        }

        let lines = read_test_log(&path);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[1], "line 2");

        cleanup_test_log(&path);
        assert!(!path.exists());
    }

    #[test]
    fn test_wait_for_log_line() {
        let path = create_test_log();

        // Spawn thread to write after delay
        let path_clone = path.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            let mut file = fs::OpenOptions::new()
                .append(true)
                .open(&path_clone)
                .expect("open for append");
            writeln!(file, "marker: found it").unwrap();
        });

        let result = wait_for_log_line(&path, Duration::from_secs(1), |line| {
            line.contains("marker:")
        });

        assert!(result.is_some());
        assert!(result.unwrap().contains("found it"));

        cleanup_test_log(&path);
    }
}
