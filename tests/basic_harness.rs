//! Basic demonstration of the kitty harness functionality.

#![allow(unused_crate_dependencies)]

use std::path::PathBuf;
use std::time::Duration;

use kitty_test_harness::{KeyPress, kitty_send_keys, with_kitty_capture};
use kitty_test_harness::{wait_for_ready_marker, wait_for_screen_text, wait_for_screen_text_clean};
use termwiz::input::KeyCode;

#[test]
#[ignore = "example test"]
fn basic_echo_capture() {
	let working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

	let output = with_kitty_capture(&working_dir, "bash", |kitty| {
		wait_for_ready_marker(kitty);
		kitty.send_text("echo 'Hello from kitty harness'\n");
		wait_for_screen_text(kitty, Duration::from_secs(2), |text| {
			text.contains("Hello from kitty harness")
		})
	});

	assert!(
		output.contains("Hello from kitty harness"),
		"Expected echo output to appear in screen capture"
	);
}

#[test]
#[ignore = "example test"]
fn key_press_navigation() {
	let working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

	with_kitty_capture(&working_dir, "bash", |kitty| {
		wait_for_ready_marker(kitty);
		kitty.send_text("printf 'Line 1\\nLine 2\\nLine 3\\n'\n");
		std::thread::sleep(Duration::from_millis(150));

		// Send arrow keys using macro
		kitty_send_keys!(kitty, KeyCode::UpArrow, KeyCode::UpArrow);

		let after = wait_for_screen_text(kitty, Duration::from_secs(2), |text| {
			text.contains("Line 1") && text.contains("Line 2") && text.contains("Line 3")
		});

		// The screen should contain the output
		assert!(after.contains("Line 1"));
		assert!(after.contains("Line 2"));
		assert!(after.contains("Line 3"));
	});
}

#[test]
#[ignore = "example test"]
fn ansi_stripping() {
	let working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

	with_kitty_capture(&working_dir, "bash", |kitty| {
		wait_for_ready_marker(kitty);
		kitty.send_text("printf '\\033[31mRed text\\033[0m\\n'\n");
		let (raw, clean) =
			wait_for_screen_text_clean(kitty, Duration::from_secs(2), |_raw, clean| {
				clean.contains("Red text")
			});

		// Raw output should contain escape sequences
		assert!(raw.contains("\x1b["));

		// Clean output should not contain escape sequences
		assert!(clean.contains("Red text"));
		assert!(!clean.contains("\x1b["));
	});
}

#[test]
#[ignore = "example test"]
fn key_press_with_modifiers() {
	let working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

	with_kitty_capture(&working_dir, "bash", |kitty| {
		use termwiz::input::Modifiers;

		wait_for_ready_marker(kitty);

		// Run `cat` so we can observe echoed input and stop it with Ctrl+C.
		kitty.send_text("cat\n");
		std::thread::sleep(Duration::from_millis(100));

		// Send text and wait for it to echo back from cat
		kitty.send_text("hello world\n");
		let before_ctrl_c = wait_for_screen_text(kitty, Duration::from_secs(2), |text| {
			text.contains("hello world")
		});
		assert!(
			before_ctrl_c.contains("hello world"),
			"expected cat echo to include hello world, got:\n{before_ctrl_c}"
		);

		let ctrl_c = KeyPress {
			key: KeyCode::Char('c'),
			mods: Modifiers::CTRL,
		};
		kitty_send_keys!(kitty, ctrl_c);
		let output =
			wait_for_screen_text(kitty, Duration::from_secs(2), |text| text.contains("^C"));
		assert!(output.contains("hello world"));
		assert!(
			output.contains("^C"),
			"expected ^C marker after sending Ctrl+C, got:\n{output}"
		);
	});
}
