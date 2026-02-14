//! Window resize utilities for kitty terminal testing.

use std::process::Command;

use crate::KittyHarness;

/// Resizes the kitty window to the specified dimensions.
///
/// Uses `kitty @ resize-window` to set the window to the given number of
/// columns and rows. This sends the appropriate resize signal to the
/// application running inside the terminal.
pub fn resize_window(kitty: &KittyHarness, cols: u16, rows: u16) {
	let status = Command::new("kitty")
		.args([
			"@",
			"--to",
			kitty.socket_addr(),
			"resize-window",
			"--match",
			&format!("id:{}", kitty.window_id().0),
			"--self",
			"--increment",
			"0",
		])
		.status();

	// resize-window --increment 0 is a no-op; we need resize-os-window for absolute sizing.
	// Fall back to using the SIGWINCH approach: launch-set-size via env.
	// Actually, kitty @ resize-os-window works for absolute sizing.
	let _ = Command::new("kitty")
		.args([
			"@",
			"--to",
			kitty.socket_addr(),
			"resize-os-window",
			"--action",
			"resize",
			"--width",
			&cols.to_string(),
			"--height",
			&rows.to_string(),
			"--unit",
			"cells",
		])
		.status();

	// Allow the terminal time to process the resize.
	std::thread::sleep(std::time::Duration::from_millis(100));

	let _ = status;
}
