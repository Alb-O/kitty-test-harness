//! Recording replay for kitty-test-harness.
//!
//! Parses the text format produced by xeno's `EventRecorder` and replays it
//! against a running kitty harness instance.
//!
//! # Format
//!
//! ```text
//! # comments
//! j                      # key event
//! C-x                    # key with modifier
//!                        # blank line = batch boundary
//! mouse:press left 10,5
//! paste:aGVsbG8=
//! resize:120x50
//! focus:in
//! ```

use std::time::Duration;

use crate::KittyHarness;
use crate::utils::mouse::{MouseButton, ScrollDirection, encode_mouse_drag, encode_mouse_move, encode_mouse_press, encode_mouse_release, encode_mouse_scroll};
use crate::utils::resize::resize_window;

/// A parsed replay event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayEvent {
	/// A batch of key names to be sent as a single `send_text` call.
	KeyBatch(Vec<String>),
	/// Mouse press event.
	MousePress {
		/// Button pressed.
		button: MouseButton,
		/// Column (0-based).
		col: u16,
		/// Row (0-based).
		row: u16,
	},
	/// Mouse release event.
	MouseRelease {
		/// Column (0-based).
		col: u16,
		/// Row (0-based).
		row: u16,
	},
	/// Mouse drag event.
	MouseDrag {
		/// Button held.
		button: MouseButton,
		/// Column (0-based).
		col: u16,
		/// Row (0-based).
		row: u16,
	},
	/// Mouse scroll event.
	MouseScroll {
		/// Scroll direction.
		direction: ScrollDirection,
		/// Column (0-based).
		col: u16,
		/// Row (0-based).
		row: u16,
	},
	/// Mouse move event.
	MouseMove {
		/// Column (0-based).
		col: u16,
		/// Row (0-based).
		row: u16,
	},
	/// Paste content (raw string, decoded from base64).
	Paste(String),
	/// Window resize.
	Resize {
		/// Columns.
		cols: u16,
		/// Rows.
		rows: u16,
	},
	/// Focus gained.
	FocusIn,
	/// Focus lost.
	FocusOut,
}

/// Parses a recording file into replay events.
///
/// Consecutive key lines are grouped into `KeyBatch` events. Blank lines
/// and non-key events flush the current key batch.
pub fn parse_recording(input: &str) -> Vec<ReplayEvent> {
	let mut events = Vec::new();
	let mut key_batch: Vec<String> = Vec::new();

	for line in input.lines() {
		let trimmed = line.trim();

		// Comments
		if trimmed.starts_with('#') {
			continue;
		}

		// Blank line = batch boundary
		if trimmed.is_empty() {
			if !key_batch.is_empty() {
				events.push(ReplayEvent::KeyBatch(std::mem::take(&mut key_batch)));
			}
			continue;
		}

		// Non-key events
		if let Some(rest) = trimmed.strip_prefix("mouse:") {
			flush_keys(&mut key_batch, &mut events);
			if let Some(ev) = parse_mouse(rest) {
				events.push(ev);
			}
		} else if let Some(rest) = trimmed.strip_prefix("paste:") {
			flush_keys(&mut key_batch, &mut events);
			if let Some(ev) = parse_paste(rest) {
				events.push(ev);
			}
		} else if let Some(rest) = trimmed.strip_prefix("resize:") {
			flush_keys(&mut key_batch, &mut events);
			if let Some(ev) = parse_resize(rest) {
				events.push(ev);
			}
		} else if let Some(rest) = trimmed.strip_prefix("focus:") {
			flush_keys(&mut key_batch, &mut events);
			match rest {
				"in" => events.push(ReplayEvent::FocusIn),
				"out" => events.push(ReplayEvent::FocusOut),
				_ => {}
			}
		} else {
			// Key event
			key_batch.push(trimmed.to_string());
		}
	}

	// Flush trailing keys
	if !key_batch.is_empty() {
		events.push(ReplayEvent::KeyBatch(key_batch));
	}

	events
}

fn flush_keys(batch: &mut Vec<String>, events: &mut Vec<ReplayEvent>) {
	if !batch.is_empty() {
		events.push(ReplayEvent::KeyBatch(std::mem::take(batch)));
	}
}

fn parse_mouse(rest: &str) -> Option<ReplayEvent> {
	let mut parts = rest.splitn(3, ' ');
	let kind = parts.next()?;

	match kind {
		"press" => {
			let button = parse_button(parts.next()?)?;
			let (col, row) = parse_coords(parts.next().unwrap_or(""))?;
			Some(ReplayEvent::MousePress { button, col, row })
		}
		"release" => {
			let coords_str = parts.next()?;
			let (col, row) = parse_coords(coords_str)?;
			Some(ReplayEvent::MouseRelease { col, row })
		}
		"drag" => {
			let button = parse_button(parts.next()?)?;
			let (col, row) = parse_coords(parts.next().unwrap_or(""))?;
			Some(ReplayEvent::MouseDrag { button, col, row })
		}
		"scroll" => {
			let direction = parse_direction(parts.next()?)?;
			let (col, row) = parse_coords(parts.next().unwrap_or(""))?;
			Some(ReplayEvent::MouseScroll { direction, col, row })
		}
		"move" => {
			let (col, row) = parse_coords(parts.next()?)?;
			Some(ReplayEvent::MouseMove { col, row })
		}
		_ => None,
	}
}

fn parse_button(s: &str) -> Option<MouseButton> {
	match s {
		"left" => Some(MouseButton::Left),
		"right" => Some(MouseButton::Right),
		"middle" => Some(MouseButton::Middle),
		_ => None,
	}
}

fn parse_direction(s: &str) -> Option<ScrollDirection> {
	match s {
		"up" => Some(ScrollDirection::Up),
		"down" => Some(ScrollDirection::Down),
		"left" => Some(ScrollDirection::Left),
		"right" => Some(ScrollDirection::Right),
		_ => None,
	}
}

fn parse_coords(s: &str) -> Option<(u16, u16)> {
	// Format: "col,row" possibly followed by " modifiers"
	let coord_part = s.split(' ').next()?;
	let (col_str, row_str) = coord_part.split_once(',')?;
	let col = col_str.parse().ok()?;
	let row = row_str.parse().ok()?;
	Some((col, row))
}

fn parse_paste(rest: &str) -> Option<ReplayEvent> {
	use base64::Engine;
	let bytes = base64::engine::general_purpose::STANDARD.decode(rest).ok()?;
	let content = String::from_utf8(bytes).ok()?;
	Some(ReplayEvent::Paste(content))
}

fn parse_resize(rest: &str) -> Option<ReplayEvent> {
	let (cols_str, rows_str) = rest.split_once('x')?;
	let cols = cols_str.parse().ok()?;
	let rows = rows_str.parse().ok()?;
	Some(ReplayEvent::Resize { cols, rows })
}

/// Replay timing configuration.
pub struct ReplayTiming {
	/// Pause between batches (separated by blank lines in the recording).
	pub batch_pause: Duration,
	/// Delay between individual keys within a batch. When non-zero, keys
	/// are sent one at a time instead of concatenated into a single
	/// `send_text` call, giving the application time to process each key.
	pub key_delay: Duration,
}

impl ReplayTiming {
	/// Batched replay with no per-key delay.
	pub fn batched(batch_pause: Duration) -> Self {
		Self {
			batch_pause,
			key_delay: Duration::ZERO,
		}
	}

	/// Per-key replay where each key is sent individually with a delay.
	pub fn per_key(key_delay: Duration) -> Self {
		Self {
			batch_pause: key_delay,
			key_delay,
		}
	}
}

/// Replays parsed events against a kitty harness.
///
/// Key batches are encoded using termwiz. With a zero `key_delay`, each
/// batch is sent as a single `send_text` call. With a non-zero `key_delay`,
/// keys are sent individually with a pause between each one.
pub fn replay(kitty: &KittyHarness, events: &[ReplayEvent], timing: ReplayTiming) {
	use termwiz::escape::csi::KittyKeyboardFlags;
	use termwiz::input::{KeyCodeEncodeModes, KeyboardEncoding};

	let modes = KeyCodeEncodeModes {
		encoding: KeyboardEncoding::Kitty(KittyKeyboardFlags::empty()),
		application_cursor_keys: false,
		newline_mode: false,
		modify_other_keys: None,
	};

	for event in events {
		match event {
			ReplayEvent::KeyBatch(keys) => {
				if timing.key_delay.is_zero() {
					// Send entire batch as one string.
					let mut encoded = String::new();
					for key_name in keys {
						if let Some(e) = encode_key_name(key_name, modes) {
							encoded.push_str(&e);
						}
					}
					if !encoded.is_empty() {
						kitty.send_text(&encoded);
					}
				} else {
					// Send each key individually with a delay.
					for key_name in keys {
						if let Some(e) = encode_key_name(key_name, modes) {
							kitty.send_text(&e);
							std::thread::sleep(timing.key_delay);
						}
					}
				}
				std::thread::sleep(timing.batch_pause);
			}
			ReplayEvent::MousePress { button, col, row } => {
				kitty.send_text(&encode_mouse_press(*button, *col, *row));
			}
			ReplayEvent::MouseRelease { col, row } => {
				// Use Left button for release encoding (button doesn't matter for SGR release trailer)
				kitty.send_text(&encode_mouse_release(MouseButton::Left, *col, *row));
			}
			ReplayEvent::MouseDrag { button, col, row } => {
				kitty.send_text(&encode_mouse_drag(*button, *col, *row));
			}
			ReplayEvent::MouseScroll { direction, col, row } => {
				kitty.send_text(&encode_mouse_scroll(*direction, *col, *row));
			}
			ReplayEvent::MouseMove { col, row } => {
				kitty.send_text(&encode_mouse_move(*col, *row));
			}
			ReplayEvent::Paste(content) => {
				// Bracketed paste: ESC[200~ ... ESC[201~
				let paste = format!("\x1b[200~{content}\x1b[201~");
				kitty.send_text(&paste);
			}
			ReplayEvent::Resize { cols, rows } => {
				resize_window(kitty, *cols, *rows);
			}
			ReplayEvent::FocusIn => {
				// Focus in: ESC[I
				kitty.send_text("\x1b[I");
			}
			ReplayEvent::FocusOut => {
				// Focus out: ESC[O
				kitty.send_text("\x1b[O");
			}
		}
	}
}

/// Encodes a key name (from the recording format) to a terminal escape sequence.
///
/// Parses the `C-A-S-<code>` notation and encodes via termwiz.
fn encode_key_name(name: &str, modes: termwiz::input::KeyCodeEncodeModes) -> Option<String> {
	use termwiz::input::{KeyCode, Modifiers};

	let mut remaining = name;
	let mut mods = Modifiers::NONE;

	// Parse modifier prefixes
	loop {
		if let Some(rest) = remaining.strip_prefix("C-") {
			mods |= Modifiers::CTRL;
			remaining = rest;
		} else if let Some(rest) = remaining.strip_prefix("A-") {
			mods |= Modifiers::ALT;
			remaining = rest;
		} else if let Some(rest) = remaining.strip_prefix("S-") {
			mods |= Modifiers::SHIFT;
			remaining = rest;
		} else {
			break;
		}
	}

	let keycode = match remaining {
		"esc" => KeyCode::Escape,
		"enter" | "ret" => KeyCode::Enter,
		"tab" => KeyCode::Tab,
		"backtab" => KeyCode::Tab, // backtab is shift+tab
		"backspace" | "bs" => KeyCode::Backspace,
		"del" | "delete" => KeyCode::Delete,
		"insert" | "ins" => KeyCode::Insert,
		"home" => KeyCode::Home,
		"end" => KeyCode::End,
		"pageup" => KeyCode::PageUp,
		"pagedown" => KeyCode::PageDown,
		"up" => KeyCode::UpArrow,
		"down" => KeyCode::DownArrow,
		"left" => KeyCode::LeftArrow,
		"right" => KeyCode::RightArrow,
		"space" => KeyCode::Char(' '),
		s if s.starts_with('F') || s.starts_with('f') => {
			let n: u8 = s[1..].parse().ok()?;
			KeyCode::Function(n)
		}
		s if s.chars().count() == 1 => KeyCode::Char(s.chars().next().unwrap()),
		_ => return None,
	};

	// backtab implies shift
	if remaining == "backtab" {
		mods |= Modifiers::SHIFT;
	}

	keycode.encode(mods, modes, true).ok()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_key_batch() {
		let input = "j\nk\nC-x\n";
		let events = parse_recording(input);
		assert_eq!(events, vec![ReplayEvent::KeyBatch(vec!["j".into(), "k".into(), "C-x".into()])]);
	}

	#[test]
	fn parse_batch_boundary() {
		let input = "j\n\nk\n";
		let events = parse_recording(input);
		assert_eq!(events, vec![ReplayEvent::KeyBatch(vec!["j".into()]), ReplayEvent::KeyBatch(vec!["k".into()]),]);
	}

	#[test]
	fn parse_mouse_events() {
		let input = "mouse:press left 10,5\nmouse:release 10,5\nmouse:scroll up 3,7\n";
		let events = parse_recording(input);
		assert_eq!(
			events,
			vec![
				ReplayEvent::MousePress {
					button: MouseButton::Left,
					col: 10,
					row: 5
				},
				ReplayEvent::MouseRelease { col: 10, row: 5 },
				ReplayEvent::MouseScroll {
					direction: ScrollDirection::Up,
					col: 3,
					row: 7
				},
			]
		);
	}

	#[test]
	fn parse_paste() {
		let input = "paste:aGVsbG8gd29ybGQ=\n";
		let events = parse_recording(input);
		assert_eq!(events, vec![ReplayEvent::Paste("hello world".into())]);
	}

	#[test]
	fn parse_resize() {
		let input = "resize:120x50\n";
		let events = parse_recording(input);
		assert_eq!(events, vec![ReplayEvent::Resize { cols: 120, rows: 50 }]);
	}

	#[test]
	fn parse_focus() {
		let input = "focus:in\nfocus:out\n";
		let events = parse_recording(input);
		assert_eq!(events, vec![ReplayEvent::FocusIn, ReplayEvent::FocusOut]);
	}

	#[test]
	fn parse_comments_ignored() {
		let input = "# this is a comment\nj\n";
		let events = parse_recording(input);
		assert_eq!(events, vec![ReplayEvent::KeyBatch(vec!["j".into()])]);
	}

	#[test]
	fn non_key_flushes_batch() {
		let input = "j\nk\nfocus:in\nl\n";
		let events = parse_recording(input);
		assert_eq!(
			events,
			vec![
				ReplayEvent::KeyBatch(vec!["j".into(), "k".into()]),
				ReplayEvent::FocusIn,
				ReplayEvent::KeyBatch(vec!["l".into()]),
			]
		);
	}

	#[test]
	fn encode_simple_char() {
		use termwiz::escape::csi::KittyKeyboardFlags;
		use termwiz::input::{KeyCodeEncodeModes, KeyboardEncoding};
		let modes = KeyCodeEncodeModes {
			encoding: KeyboardEncoding::Kitty(KittyKeyboardFlags::empty()),
			application_cursor_keys: false,
			newline_mode: false,
			modify_other_keys: None,
		};
		assert_eq!(encode_key_name("j", modes), Some("j".into()));
		assert_eq!(encode_key_name("esc", modes), Some("\x1b".into()));
	}
}
