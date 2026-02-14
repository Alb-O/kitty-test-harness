//! Mouse event encoding for terminal testing.
//!
//! This module provides utilities for sending mouse events to terminal applications
//! via SGR mouse encoding (mode 1006), which is the most widely supported extended
//! mouse protocol.
//!
//! # Mouse Event Encoding
//!
//! SGR mouse encoding uses the format: `\x1b[<Cb;Cx;CyM` for press and `\x1b[<Cb;Cx;Cym` for release
//! Where:
//! - Cb = button code (0=left, 1=middle, 2=right, 32+motion, 64+scroll)
//! - Cx = column (1-based)
//! - Cy = row (1-based)
//! - M = press, m = release
//!
//! # Example
//!
//! ```ignore
//! use kitty_test_harness::utils::mouse::{MouseButton, send_mouse_click, send_mouse_drag};
//!
//! // Click at position (10, 5)
//! send_mouse_click(kitty, MouseButton::Left, 10, 5);
//!
//! // Drag from (10, 5) to (20, 5)
//! send_mouse_drag(kitty, MouseButton::Left, 10, 5, 20, 5);
//! ```

use crate::KittyHarness;

/// Mouse button identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
	/// Left mouse button (button 0).
	Left,
	/// Middle mouse button (button 1).
	Middle,
	/// Right mouse button (button 2).
	Right,
}

impl MouseButton {
	/// Returns the SGR button code for this button.
	fn code(self) -> u8 {
		match self {
			MouseButton::Left => 0,
			MouseButton::Middle => 1,
			MouseButton::Right => 2,
		}
	}
}

/// Encodes a mouse press event in SGR format.
///
/// SGR format: `\x1b[<Cb;Cx;CyM`
/// - Cb: button code
/// - Cx: column (1-based)
/// - Cy: row (1-based)
/// - M: press indicator
pub fn encode_mouse_press(button: MouseButton, col: u16, row: u16) -> String {
	// SGR uses 1-based coordinates
	let col = col + 1;
	let row = row + 1;
	format!("\x1b[<{};{};{}M", button.code(), col, row)
}

/// Encodes a mouse release event in SGR format.
///
/// SGR format: `\x1b[<Cb;Cx;Cym`
/// - Cb: button code
/// - Cx: column (1-based)
/// - Cy: row (1-based)
/// - m: release indicator
///
/// Release events keep the same button code as press and change the trailer
/// from `M` to `m`.
pub fn encode_mouse_release(button: MouseButton, col: u16, row: u16) -> String {
	let col = col + 1;
	let row = row + 1;
	format!("\x1b[<{};{};{}m", button.code(), col, row)
}

/// Encodes a mouse drag (motion with button held) event in SGR format.
///
/// Motion events have bit 5 (32) added to the button code.
pub fn encode_mouse_drag(button: MouseButton, col: u16, row: u16) -> String {
	let col = col + 1;
	let row = row + 1;
	let code = button.code() + 32; // Add motion flag
	format!("\x1b[<{};{};{}M", code, col, row)
}

/// Encodes a mouse move (motion without button) event in SGR format.
///
/// Move events use button code 35 (32 + 3, where 3 indicates no button).
pub fn encode_mouse_move(col: u16, row: u16) -> String {
	let col = col + 1;
	let row = row + 1;
	format!("\x1b[<35;{};{}M", col, row)
}

/// Sends a mouse click (press + release) at the specified position.
///
/// Coordinates are 0-based (will be converted to 1-based for SGR).
pub fn send_mouse_click(kitty: &KittyHarness, button: MouseButton, col: u16, row: u16) {
	kitty.send_text(&encode_mouse_press(button, col, row));
	std::thread::sleep(std::time::Duration::from_millis(10));
	kitty.send_text(&encode_mouse_release(button, col, row));
}

/// Sends a mouse press event at the specified position.
pub fn send_mouse_press(kitty: &KittyHarness, button: MouseButton, col: u16, row: u16) {
	kitty.send_text(&encode_mouse_press(button, col, row));
}

/// Sends a mouse release event at the specified position.
pub fn send_mouse_release(kitty: &KittyHarness, button: MouseButton, col: u16, row: u16) {
	kitty.send_text(&encode_mouse_release(button, col, row));
}

/// Sends a mouse move event at the specified position.
pub fn send_mouse_move(kitty: &KittyHarness, col: u16, row: u16) {
	kitty.send_text(&encode_mouse_move(col, row));
}

/// Sends a complete mouse drag operation from start to end position.
///
/// This sends:
/// 1. Press at start position
/// 2. Drag events along the path (currently just start and end)
/// 3. Release at end position
pub fn send_mouse_drag(kitty: &KittyHarness, button: MouseButton, start_col: u16, start_row: u16, end_col: u16, end_row: u16) {
	// Press at start
	kitty.send_text(&encode_mouse_press(button, start_col, start_row));
	std::thread::sleep(std::time::Duration::from_millis(10));

	// Drag to end
	kitty.send_text(&encode_mouse_drag(button, end_col, end_row));
	std::thread::sleep(std::time::Duration::from_millis(10));

	// Release at end
	kitty.send_text(&encode_mouse_release(button, end_col, end_row));
}

/// Sends a mouse drag operation with intermediate steps.
///
/// This is useful for testing drag behavior that depends on intermediate positions.
pub fn send_mouse_drag_with_steps(kitty: &KittyHarness, button: MouseButton, start_col: u16, start_row: u16, end_col: u16, end_row: u16, steps: u16) {
	// Press at start
	kitty.send_text(&encode_mouse_press(button, start_col, start_row));
	std::thread::sleep(std::time::Duration::from_millis(10));

	// Interpolate intermediate positions
	for i in 1..=steps {
		let t = i as f32 / steps as f32;
		let col = start_col as f32 + (end_col as f32 - start_col as f32) * t;
		let row = start_row as f32 + (end_row as f32 - start_row as f32) * t;
		kitty.send_text(&encode_mouse_drag(button, col as u16, row as u16));
		std::thread::sleep(std::time::Duration::from_millis(10));
	}

	// Release at end
	kitty.send_text(&encode_mouse_release(button, end_col, end_row));
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_encode_mouse_press() {
		// Position (0, 0) should encode as (1, 1)
		assert_eq!(encode_mouse_press(MouseButton::Left, 0, 0), "\x1b[<0;1;1M");
		// Position (9, 4) should encode as (10, 5)
		assert_eq!(encode_mouse_press(MouseButton::Left, 9, 4), "\x1b[<0;10;5M");
		// Right button
		assert_eq!(encode_mouse_press(MouseButton::Right, 5, 5), "\x1b[<2;6;6M");
	}

	#[test]
	fn test_encode_mouse_release() {
		assert_eq!(encode_mouse_release(MouseButton::Left, 0, 0), "\x1b[<0;1;1m");
	}

	#[test]
	fn test_encode_mouse_release_per_button() {
		assert_eq!(encode_mouse_release(MouseButton::Left, 2, 3), "\x1b[<0;3;4m");
		assert_eq!(encode_mouse_release(MouseButton::Middle, 2, 3), "\x1b[<1;3;4m");
		assert_eq!(encode_mouse_release(MouseButton::Right, 2, 3), "\x1b[<2;3;4m");
	}

	#[test]
	fn test_encode_mouse_drag() {
		// Drag has motion flag (32) added
		assert_eq!(encode_mouse_drag(MouseButton::Left, 0, 0), "\x1b[<32;1;1M");
	}

	#[test]
	fn test_encode_mouse_move() {
		// Move uses code 35 (32 + 3)
		assert_eq!(encode_mouse_move(0, 0), "\x1b[<35;1;1M");
	}
}
