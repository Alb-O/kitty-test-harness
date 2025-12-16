use crate::KittyHarness;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Wait until the screen text satisfies the given predicate or the timeout is reached.
pub fn wait_for_screen_text(
	kitty: &KittyHarness,
	timeout: Duration,
	predicate: impl Fn(&str) -> bool,
) -> String {
	let start = Instant::now();
	let mut last = String::new();
	while start.elapsed() <= timeout {
		last = kitty.screen_text();
		if predicate(&last) {
			break;
		}
		std::thread::sleep(Duration::from_millis(50));
	}
	last
}

static READY_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Wait for a unique ready marker to appear in the kitty harness output.
pub fn wait_for_ready_marker(kitty: &KittyHarness) {
	let idx = READY_COUNTER.fetch_add(1, Ordering::Relaxed);
	let marker = format!("__KITTY_READY_{idx}__");
	// Print a unique marker and wait until it shows up in the captured output.
	kitty.send_text(&format!("printf '{}\\n'\n", marker));
	let _ = wait_for_screen_text(kitty, Duration::from_secs(5), |text| text.contains(&marker));
}

/// Wait until the cleaned screen text satisfies the given predicate or the timeout is reached.
pub fn wait_for_screen_text_clean(
	kitty: &KittyHarness,
	timeout: Duration,
	predicate: impl Fn(&str, &str) -> bool,
) -> (String, String) {
	let start = Instant::now();
	let mut last = (String::new(), String::new());
	while start.elapsed() <= timeout {
		last = kitty.screen_text_clean();
		if predicate(&last.0, &last.1) {
			break;
		}
		std::thread::sleep(Duration::from_millis(50));
	}
	last
}
