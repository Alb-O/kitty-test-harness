//! Common testing patterns and helpers for terminal application testing.
//!
//! This module provides utilities for common scenarios encountered when testing
//! terminal applications with the kitty harness.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn shell_single_quote(value: &str) -> String {
	format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn is_valid_env_key(key: &str) -> bool {
	let mut chars = key.chars();
	let Some(first) = chars.next() else {
		return false;
	};
	if !(first == '_' || first.is_ascii_alphabetic()) {
		return false;
	}
	chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

/// Creates a mock executable script that logs its invocation arguments.
///
/// This is useful for testing commands that invoke external programs (like `kitty @`).
/// The mock script writes the current directory and all arguments to the specified log file.
///
/// # Arguments
/// * `log_path` - Path where invocation logs will be written
/// * `output_dir` - Directory where the mock script will be created
///
/// # Returns
/// Path to the created mock script
///
/// # Example
/// ```no_run
/// use kitty_test_harness::utils::patterns::create_mock_executable;
/// use std::path::PathBuf;
///
/// let log_path = PathBuf::from("/tmp/mock-log.txt");
/// let output_dir = PathBuf::from("/tmp");
/// let mock = create_mock_executable(&log_path, &output_dir);
///
/// // Run your test that invokes the mock...
///
/// // Then check the log for expected arguments
/// let contents = std::fs::read_to_string(&log_path).unwrap();
/// assert!(contents.contains("--expected-arg"));
/// ```
pub fn create_mock_executable(log_path: &Path, output_dir: &Path) -> PathBuf {
	let _ = fs::create_dir_all(output_dir);
	let mock_path = output_dir.join("mock-executable.sh");
	let escaped_log_path = shell_single_quote(&log_path.display().to_string());
	let script = format!("#!/bin/sh\nprintf '%s\\n' \"$PWD\" \"$@\" >> {}\n", escaped_log_path);
	fs::write(&mock_path, script).expect("write mock executable");
	let mut perms = fs::metadata(&mock_path).expect("mock perms").permissions();
	perms.set_mode(0o755);
	fs::set_permissions(&mock_path, perms).expect("chmod mock");
	mock_path
}

/// Creates a wrapper script that sets environment variables before running a command.
///
/// This is useful when you need to pass environment variables to a process launched
/// inside kitty, since the harness can only pass env vars to the kitty process itself,
/// not necessarily to programs launched via `bash -lc`.
///
/// # Arguments
/// * `env_vars` - Slice of (key, value) pairs for environment variables to set
/// * `target_cmd` - The command to execute after setting env vars
/// * `output_dir` - Directory where the wrapper script will be created
///
/// # Returns
/// Path to the created wrapper script
///
/// # Example
/// ```no_run
/// use kitty_test_harness::utils::patterns::create_env_wrapper;
/// use std::path::PathBuf;
///
/// let env_vars = &[
///     ("MY_VAR", "/path/to/something"),
///     ("DEBUG", "1"),
/// ];
/// let wrapper = create_env_wrapper(env_vars, "/usr/bin/my-app", &PathBuf::from("/tmp"));
///
/// // Use wrapper.display() as the command for kitty
/// ```
pub fn create_env_wrapper(env_vars: &[(&str, &str)], target_cmd: &str, output_dir: &Path) -> PathBuf {
	let _ = fs::create_dir_all(output_dir);
	let wrapper = output_dir.join("env-wrapper.sh");

	let exports: String = env_vars
		.iter()
		.map(|(k, v)| {
			assert!(is_valid_env_key(k), "invalid env var name: {k}");
			format!("export {}={}\n", k, shell_single_quote(v))
		})
		.collect();

	let script = format!("#!/bin/sh\n{}exec {} \"$@\"\n", exports, shell_single_quote(target_cmd));

	fs::write(&wrapper, script).expect("write env wrapper");
	let mut perms = fs::metadata(&wrapper).expect("wrapper perms").permissions();
	perms.set_mode(0o755);
	fs::set_permissions(&wrapper, perms).expect("chmod wrapper");
	wrapper
}

/// Parses a mock log file into lines, useful for asserting on command arguments.
///
/// The first line is typically the working directory, followed by one argument per line.
///
/// # Example
/// ```no_run
/// use kitty_test_harness::utils::patterns::parse_mock_log;
/// use std::path::PathBuf;
///
/// let args = parse_mock_log(&PathBuf::from("/tmp/mock-log.txt")).unwrap();
/// assert!(args.iter().any(|a| a == "--cwd"));
/// ```
pub fn parse_mock_log(log_path: &Path) -> std::io::Result<Vec<String>> {
	let contents = fs::read_to_string(log_path)?;
	Ok(contents.lines().map(String::from).collect())
}

/// Waits for a file to exist, with a configurable number of retries.
///
/// Useful for waiting on mock logs or output files that are created asynchronously.
///
/// # Arguments
/// * `path` - Path to wait for
/// * `retries` - Number of 50ms retries before giving up
///
/// # Returns
/// `true` if the file exists, `false` if retries exhausted
pub fn wait_for_file(path: &Path, retries: usize) -> bool {
	for _ in 0..retries {
		if path.exists() {
			return true;
		}
		std::thread::sleep(std::time::Duration::from_millis(50));
	}
	path.exists()
}

#[cfg(test)]
mod tests {
	use std::env::temp_dir;
	use std::sync::atomic::{AtomicUsize, Ordering};

	use super::*;

	fn temp_test_dir(label: &str) -> PathBuf {
		static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
		let idx = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
		let dir = temp_dir().join(format!("kitty-test-patterns-{label}-{idx}"));
		let _ = fs::remove_dir_all(&dir);
		fs::create_dir_all(&dir).expect("create test temp dir");
		dir
	}

	#[test]
	fn test_create_mock_executable() {
		let tmp = temp_test_dir("mock");
		let log = tmp.join("test-mock.log");
		let _ = fs::remove_file(&log);

		let mock = create_mock_executable(&log, &tmp);
		assert!(mock.exists());

		// Verify it's executable
		let perms = fs::metadata(&mock).unwrap().permissions();
		assert!(perms.mode() & 0o111 != 0);
	}

	#[test]
	fn test_create_env_wrapper() {
		let tmp = temp_test_dir("wrapper-basic");
		let wrapper = create_env_wrapper(&[("FOO", "bar"), ("BAZ", "qux")], "/bin/true", &tmp);

		let contents = fs::read_to_string(&wrapper).unwrap();
		assert!(contents.contains("export FOO='bar'"));
		assert!(contents.contains("export BAZ='qux'"));
		assert!(contents.contains("exec '/bin/true'"));
	}

	#[test]
	fn test_create_env_wrapper_escapes_values_and_target() {
		let tmp = temp_test_dir("wrapper-escaped");
		let wrapper = create_env_wrapper(&[("WITH_SPACE", "hello world"), ("WITH_QUOTE", "it's \"$HOME\"")], "/tmp/my app/bin", &tmp);

		let contents = fs::read_to_string(&wrapper).unwrap();
		assert!(contents.contains("export WITH_SPACE='hello world'"));
		assert!(contents.contains("export WITH_QUOTE='it'\"'\"'s \"$HOME\"'"));
		assert!(contents.contains("exec '/tmp/my app/bin' \"$@\""));
	}

	#[test]
	fn test_create_mock_executable_escapes_log_path() {
		let tmp = temp_test_dir("mock-escaped");
		let log = tmp.join("odd ' path.log");
		let mock = create_mock_executable(&log, &tmp);

		let contents = fs::read_to_string(&mock).unwrap();
		assert!(contents.contains("'\"'\"'"));
	}

	#[test]
	#[should_panic(expected = "invalid env var name")]
	fn test_create_env_wrapper_rejects_invalid_env_key() {
		let tmp = temp_test_dir("wrapper-invalid-key");
		let _ = create_env_wrapper(&[("BAD-KEY", "value")], "/bin/true", &tmp);
	}
}
