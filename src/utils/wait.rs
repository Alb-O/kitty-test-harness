use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::KittyHarness;

/// Error returned when waiting for screen content times out.
#[derive(Debug, Clone)]
pub struct WaitTimeout {
	/// Elapsed time before timeout was returned.
	pub elapsed: Duration,
	/// Configured timeout duration.
	pub timeout: Duration,
	/// Last captured raw screen text.
	pub last_raw: String,
	/// Last captured cleaned screen text, if applicable.
	pub last_clean: Option<String>,
}

impl WaitTimeout {
	fn raw(elapsed: Duration, timeout: Duration, last_raw: String) -> Self {
		Self {
			elapsed,
			timeout,
			last_raw,
			last_clean: None,
		}
	}

	fn clean(elapsed: Duration, timeout: Duration, last_raw: String, last_clean: String) -> Self {
		Self {
			elapsed,
			timeout,
			last_raw,
			last_clean: Some(last_clean),
		}
	}
}

impl fmt::Display for WaitTimeout {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "timed out after {:?} (configured timeout: {:?})", self.elapsed, self.timeout)
	}
}

impl Error for WaitTimeout {}

/// Wait until the screen text satisfies the given predicate or the timeout is reached.
pub fn wait_for_screen_text(kitty: &KittyHarness, timeout: Duration, predicate: impl Fn(&str) -> bool) -> String {
	wait_for_screen_text_or_timeout(kitty, timeout, predicate).unwrap_or_else(|err| err.last_raw)
}

/// Wait until the screen text satisfies the given predicate or return a timeout error.
pub fn wait_for_screen_text_or_timeout(kitty: &KittyHarness, timeout: Duration, predicate: impl Fn(&str) -> bool) -> Result<String, WaitTimeout> {
	let start = Instant::now();

	loop {
		let last = kitty.screen_text();
		if predicate(&last) {
			return Ok(last);
		}

		let elapsed = start.elapsed();
		if elapsed > timeout {
			return Err(WaitTimeout::raw(elapsed, timeout, last));
		}

		std::thread::sleep(Duration::from_millis(50));
	}
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
pub fn wait_for_screen_text_clean(kitty: &KittyHarness, timeout: Duration, predicate: impl Fn(&str, &str) -> bool) -> (String, String) {
	wait_for_screen_text_clean_or_timeout(kitty, timeout, predicate).unwrap_or_else(|err| (err.last_raw, err.last_clean.unwrap_or_default()))
}

/// Wait until cleaned screen text satisfies the predicate or return a timeout error.
pub fn wait_for_screen_text_clean_or_timeout(
	kitty: &KittyHarness,
	timeout: Duration,
	predicate: impl Fn(&str, &str) -> bool,
) -> Result<(String, String), WaitTimeout> {
	let start = Instant::now();

	loop {
		let last = kitty.screen_text_clean();
		if predicate(&last.0, &last.1) {
			return Ok(last);
		}

		let elapsed = start.elapsed();
		if elapsed > timeout {
			return Err(WaitTimeout::clean(elapsed, timeout, last.0, last.1));
		}

		std::thread::sleep(Duration::from_millis(50));
	}
}

/// Wait until the cleaned screen text contains the provided substring.
pub fn wait_for_clean_contains(kitty: &KittyHarness, timeout: Duration, needle: &str) -> String {
	let (_raw, clean) = wait_for_screen_text_clean(kitty, timeout, |_raw, clean| clean.contains(needle));
	clean
}

/// Rapidly sample the screen for a duration, collecting all captured frames.
///
/// This is useful for catching transient states like animations. The function
/// captures as fast as possible without any sleep between captures.
///
/// Returns a vector of (raw, clean) screen captures with timestamps relative
/// to the start of sampling.
pub fn sample_screen_rapidly(kitty: &KittyHarness, duration: Duration) -> Vec<(Duration, String, String)> {
	let start = Instant::now();
	let mut samples = Vec::new();

	while start.elapsed() < duration {
		let (raw, clean) = kitty.screen_text_clean();
		samples.push((start.elapsed(), raw, clean));
	}

	samples
}
