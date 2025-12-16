use kitty_remote_bindings::command::{CommandOutput, Ls};
use kitty_remote_bindings::model::WindowId;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Check if we should use kitty panel (requires Wayland with layer-shell).
/// Falls back to normal window if not on Wayland or if layer-shell is unavailable.
///
/// Can be controlled with KITTY_TEST_USE_PANEL environment variable:
/// - "1" or "true": Force panel mode
/// - "0" or "false": Force normal window mode
/// - unset: Auto-detect based on environment
pub(crate) fn should_use_panel() -> bool {
	// Allow explicit override via environment variable
	if let Ok(val) = std::env::var("KITTY_TEST_USE_PANEL") {
		return val == "1" || val.eq_ignore_ascii_case("true");
	}

	// Auto-detect: Only use panel on native Wayland (not WSL)
	// WSL has WAYLAND_DISPLAY but often lacks layer-shell support
	if std::env::var("WSL_DISTRO_NAME").is_ok() || std::env::var("WSL_INTEROP").is_ok() {
		return false;
	}

	// Check if we're on Wayland with layer-shell support
	if std::env::var("WAYLAND_DISPLAY").is_ok() {
		// If XDG_SESSION_TYPE is explicitly set to wayland, use panel
		if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
			return session_type == "wayland";
		}
		// If WAYLAND_DISPLAY is set but XDG_SESSION_TYPE isn't, be conservative
		return false;
	}

	false
}

pub(crate) fn wait_for_window(socket_addr: &str) -> WindowId {
	for _ in 0..40 {
		let ls = Ls::new().to(socket_addr.to_string());
		let mut cmd: Command = (&ls).into();
		if let Ok(output) = cmd.output()
			&& let Ok(os_windows) = Ls::result(&output)
			&& let Some(id) = first_window_id(os_windows)
		{
			return id;
		}
		thread::sleep(Duration::from_millis(100));
	}
	panic!("kitty remote control not reachable or window not found");
}

pub(crate) fn first_window_id(ls: kitty_remote_bindings::model::OsWindows) -> Option<WindowId> {
	ls.0.first()
		.and_then(|os| os.tabs.first())
		.and_then(|tab| tab.windows.first())
		.map(|win| win.id)
}
