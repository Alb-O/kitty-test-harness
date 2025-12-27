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

/// Wait until the cleaned screen text contains the provided substring.
pub fn wait_for_clean_contains(kitty: &KittyHarness, timeout: Duration, needle: &str) -> String {
	let (_raw, clean) = wait_for_screen_text_clean(kitty, timeout, |_raw, clean| {
		clean.contains(needle)
	});
	clean
}

/// Rapidly sample the screen for a duration, collecting all captured frames.
///
/// This is useful for catching transient states like animations. The function
/// captures as fast as possible without any sleep between captures.
///
/// Returns a vector of (raw, clean) screen captures with timestamps relative
/// to the start of sampling.
pub fn sample_screen_rapidly(
	kitty: &KittyHarness,
	duration: Duration,
) -> Vec<(Duration, String, String)> {
	let start = Instant::now();
	let mut samples = Vec::new();

	while start.elapsed() < duration {
		let (raw, clean) = kitty.screen_text_clean();
		samples.push((start.elapsed(), raw, clean));
	}

	samples
}
