//! Harness for driving kitty terminal via remote control and capturing screen output for integration testing.
//!
//! This library provides infrastructure for automated testing of terminal-based applications
//! by programmatically controlling kitty terminal instances through the remote control protocol.
//!
//! # Overview
//!
//! The harness launches background kitty panels, sends input sequences (text and encoded keypresses),
//! and captures rendered screen contents for assertion. Screen capture supports both raw output
//! (preserving ANSI escape sequences) and stripped output (plain text only).
//!
//! # Requirements
//!
//! - kitty terminal must be available on PATH
//! - Remote control must be enabled in kitty configuration
//!
//! # Example
//!
//! ```no_run
//! use kitty_test_harness::{kitty_send_keys, with_kitty_capture};
//! use termwiz::input::KeyCode;
//! use std::path::PathBuf;
//!
//! let working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
//!
//! with_kitty_capture(&working_dir, "bash", |kitty| {
//!     kitty.send_text("echo 'test'\n");
//!     std::thread::sleep(std::time::Duration::from_millis(100));
//!     
//!     let (raw, clean) = kitty.screen_text_clean();
//!     assert!(clean.contains("test"));
//! });
//! ```

use ansi_escape_sequences::strip_ansi;
use kitty_remote_bindings::command::options::Matcher;
use kitty_remote_bindings::command::{CommandOutput, SendText};
use kitty_remote_bindings::model::WindowId;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;
use termwiz::escape::csi::KittyKeyboardFlags;
use termwiz::input::{KeyCode, KeyCodeEncodeModes, KeyboardEncoding, Modifiers};
use utils::window::{should_use_panel, wait_for_window};

pub mod utils;
pub use utils::wait::{wait_for_ready_marker, wait_for_screen_text, wait_for_screen_text_clean};

#[cfg(test)]
use insta as _;

/// Drive a kitty window via remote control and capture its contents.
pub struct KittyHarness {
	socket_addr: String,
	window_id: WindowId,
}

impl KittyHarness {
	/// Launch a background kitty panel running the provided shell command.
	pub fn launch(working_dir: &Path, command: &str) -> Self {
		let session = next_session_name();
		let socket = working_dir.join(format!("{session}.sock"));
		let socket_addr = format!("unix:{}", socket.display());

		if socket.exists() {
			let _ = std::fs::remove_file(&socket);
		}

		// Panel requires Wayland with layer-shell protocol support
		let use_panel = should_use_panel();

		if use_panel {
			// Try to launch as a background panel (requires Wayland layer-shell)
			let mut cmd = Command::new("kitty");
			let status = cmd
				.current_dir(working_dir)
				.args([
					"+kitten",
					"panel",
					"--focus-policy=not-allowed",
					"--edge=background",
					"--listen-on",
					&socket_addr,
					"--class",
					&session,
					"-o",
					"allow_remote_control=yes",
					"--detach",
					"bash",
					"--noprofile",
					"--norc",
					"-lc",
					command,
				])
				.status()
				.expect("kitty panel launch should run");
			assert!(status.success(), "kitty panel should launch");
		} else {
			// Use a normal window instead of a panel
			let mut cmd = Command::new("kitty");
			let _ = cmd
				.current_dir(working_dir)
				.args([
					"--listen-on",
					&socket_addr,
					"--class",
					&session,
					"-o",
					"allow_remote_control=yes",
					"bash",
					"--noprofile",
					"--norc",
					"-lc",
					command,
				])
				.spawn()
				.expect("kitty launch should spawn")
				.wait();
			// Give kitty a moment to create the socket
			thread::sleep(Duration::from_millis(200));
		}

		let window_id = wait_for_window(&socket_addr);

		Self {
			socket_addr,
			window_id,
		}
	}

	/// Send raw text to the kitty window (e.g., escape sequences for arrows).
	pub fn send_text(&self, text: &str) {
		let send = SendText::new(text.to_string())
			.to(self.socket_addr.clone())
			.matcher(Matcher::Id(self.window_id));
		let mut cmd: Command = (&send).into();
		let output = cmd.output().expect("kitty send-text should run");
		std::thread::sleep(Duration::from_millis(20));
		SendText::result(&output).expect("kitty send-text should succeed");
	}

	/// Capture the current screen contents as ANSI text with trailing whitespace trimmed.
	pub fn screen_text(&self) -> String {
		let output = Command::new("kitty")
			.args([
				"@",
				"--to",
				&self.socket_addr,
				"get-text",
				"--match",
				&format!("id:{}", self.window_id.0),
				"--ansi",
				"--extent",
				"screen",
			])
			.output()
			.expect("kitty get-text should run");
		assert!(
			output.status.success(),
			"kitty get-text failed: stdout: {} stderr: {}",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		);
		let raw = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
		clean_trailing_whitespace(&raw)
	}

	/// Capture the screen text and a variant with ANSI escapes stripped.
	pub fn screen_text_clean(&self) -> (String, String) {
		let raw = self.screen_text();
		let clean = strip_ansi(&raw);
		(raw, clean)
	}
}

static SESSION_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_session_name() -> String {
	let pid = std::process::id();
	let idx = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
	format!("kitty-test-{pid}-{idx}")
}

impl Drop for KittyHarness {
	fn drop(&mut self) {
		let _ = Command::new("kitty")
			.args([
				"@",
				"--to",
				&self.socket_addr,
				"close-window",
				"--match",
				&format!("id:{}", self.window_id.0),
			])
			.status();
	}
}

/// A key press plus optional modifier to encode for kitty.
#[derive(Clone, Copy, Debug)]
pub struct KeyPress {
	/// Key code to encode and send.
	pub key: KeyCode,
	/// Modifier flags to encode alongside the key.
	pub mods: Modifiers,
}

impl From<KeyCode> for KeyPress {
	fn from(key: KeyCode) -> Self {
		Self {
			key,
			mods: Modifiers::NONE,
		}
	}
}

impl From<(KeyCode, Modifiers)> for KeyPress {
	fn from((key, mods): (KeyCode, Modifiers)) -> Self {
		Self { key, mods }
	}
}

fn encode_key(key: KeyPress, modes: KeyCodeEncodeModes) -> String {
	key.key
		.encode(key.mods, modes, true)
		.expect("termwiz should encode key")
}

fn default_key_modes() -> KeyCodeEncodeModes {
	KeyCodeEncodeModes {
		encoding: KeyboardEncoding::Kitty(KittyKeyboardFlags::empty()),
		application_cursor_keys: false,
		newline_mode: false,
		modify_other_keys: None,
	}
}

/// Encode and send a sequence of key presses with custom key modes.
pub fn send_keys_with_modes(kitty: &KittyHarness, modes: KeyCodeEncodeModes, keys: &[KeyPress]) {
	for key in keys {
		kitty.send_text(&encode_key(*key, modes));
	}
}

/// Encode and send key presses with default kitty modes.
pub fn send_keys(kitty: &KittyHarness, keys: &[KeyPress]) {
	send_keys_with_modes(kitty, default_key_modes(), keys)
}

/// Launch kitty, run `command`, and let the caller drive interactions to produce a result.
pub fn with_kitty_capture<T>(
	working_dir: &Path,
	command: &str,
	driver: impl FnOnce(&KittyHarness) -> T,
) -> T {
	let harness = KittyHarness::launch(working_dir, command);
	driver(&harness)
}

/// Resolve the cargo manifest directory for the current crate.
///
/// This provides the directory containing the Cargo.toml of the test crate,
/// which can be used as a base for resolving project paths in tests.
pub fn manifest_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Encode and send a sequence of keys using termwiz's key encoder.
#[macro_export]
macro_rules! kitty_send_keys {
	($kitty:expr, $($key:expr),+ $(,)?) => {{
		$crate::send_keys($kitty, &[$($crate::__kitty_key!($key)),+]);
	}};
	($kitty:expr, modes = $modes:expr; $($key:expr),+ $(,)?) => {{
		$crate::send_keys_with_modes($kitty, $modes, &[$($crate::__kitty_key!($key)),+]);
	}};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __kitty_key {
	(($key:expr, $mods:expr)) => {
		$crate::KeyPress::from(($key, $mods))
	};
	($key:expr) => {
		$crate::KeyPress::from($key)
	};
}

/// Define a kitty snapshot test with a provided working directory binding.
#[macro_export]
macro_rules! kitty_snapshot_test {
	($name:ident, |$dir:ident| $body:block) => {
		#[test]
		fn $name() {
			let $dir = $crate::manifest_dir();
			let output: String = { $body };
			insta::assert_snapshot!(stringify!($name), output);
		}
	};
}

fn clean_trailing_whitespace(input: &str) -> String {
	let mut cleaned_lines = Vec::new();

	for line in input.lines() {
		let tokens = split_tokens(line);
		let mut keep_until = 0usize;
		for (idx, token) in tokens.iter().enumerate() {
			if matches!(token.kind, TokenKind::Text) && !token.text.trim_end().is_empty() {
				keep_until = idx + 1;
			}
		}
		let mut kept = String::new();
		for token in tokens.iter().take(keep_until) {
			kept.push_str(&token.raw);
		}
		cleaned_lines.push(kept);
	}

	while let Some(last) = cleaned_lines.last() {
		if strip_ansi(last).trim().is_empty() {
			cleaned_lines.pop();
		} else {
			break;
		}
	}

	cleaned_lines.join("\n")
}

#[derive(Clone, Debug)]
struct Token {
	kind: TokenKind,
	raw: String,
	text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TokenKind {
	Text,
	Escape,
}

fn split_tokens(line: &str) -> Vec<Token> {
	let mut out = Vec::new();
	let mut chars = line.chars().peekable();

	while let Some(ch) = chars.next() {
		if ch == '\u{1b}' {
			let mut raw = String::from(ch);
			while let Some(&next) = chars.peek() {
				raw.push(next);
				chars.next();
				if next.is_ascii_alphabetic() || next == '~' {
					break;
				}
			}
			out.push(Token {
				kind: TokenKind::Escape,
				raw,
				text: String::new(),
			});
		} else {
			let mut raw = String::from(ch);
			while let Some(&next) = chars.peek() {
				if next == '\u{1b}' {
					break;
				}
				raw.push(next);
				chars.next();
			}
			out.push(Token {
				kind: TokenKind::Text,
				text: raw.clone(),
				raw,
			});
		}
	}

	out
}
