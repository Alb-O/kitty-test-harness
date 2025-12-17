use std::process::Command;

/// Return true if kitty-driven tests should run in this environment.
/// Prints skip reasons when unavailable (e.g., missing DISPLAY or kitty binary).
pub fn require_kitty() -> bool {
	let wants_kitty = std::env::var("KITTY_TESTS").unwrap_or_default();
	if wants_kitty.is_empty() || wants_kitty == "0" || wants_kitty.eq_ignore_ascii_case("false") {
		eprintln!("skipping kitty tests: set KITTY_TESTS=1 and run under a GUI session");
		return false;
	}

	let has_display = std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok();
	if !has_display {
		eprintln!("skipping kitty tests: DISPLAY/WAYLAND_DISPLAY not set");
		return false;
	}

	let kitty_ok = Command::new("kitty").arg("--version").output().is_ok();
	if !kitty_ok {
		eprintln!("skipping kitty tests: kitty binary not found on PATH");
	}

	kitty_ok
}
