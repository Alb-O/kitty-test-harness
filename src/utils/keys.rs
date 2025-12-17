//! Terminal key encoding helpers and documentation.
//!
//! # Terminal Key Encoding Quirks
//!
//! When testing terminal applications, key encoding can be tricky. Here are common issues:
//!
//! ## Ctrl+Enter vs Ctrl+J
//!
//! Many terminals (and the underlying TTY layer) translate `Ctrl+Enter` to `Ctrl+J` (ASCII 0x0A,
//! which is Line Feed). When testing applications that need `Ctrl+Enter`, use `Ctrl+J` instead:
//!
//! ```ignore
//! // This might not work as expected through the terminal:
//! kitty_send_keys!(kitty, (KeyCode::Enter, Modifiers::CTRL));
//!
//! // Use Ctrl+J instead (same byte value, more reliable):
//! kitty_send_keys!(kitty, (KeyCode::Char('j'), Modifiers::CTRL));
//! ```
//!
//! ## Alt/Meta Keys
//!
//! Alt-modified keys are typically sent as ESC prefix + character. The harness provides
//! `send_alt_key()` for this, but you can also use the modifier directly:
//!
//! ```ignore
//! // Using the modifier (relies on keyboard encoding):
//! kitty_send_keys!(kitty, (KeyCode::Char('x'), Modifiers::ALT));
//!
//! // Using ESC prefix (more universally compatible):
//! send_alt_key(kitty, 'x');
//! ```
//!
//! ## Control Characters
//!
//! Common control character mappings:
//! - `Ctrl+A` through `Ctrl+Z`: ASCII 0x01-0x1A
//! - `Ctrl+[`: ESC (0x1B)
//! - `Ctrl+J`: Line Feed / "Enter" in some contexts (0x0A)
//! - `Ctrl+M`: Carriage Return / Enter (0x0D)
//!
//! ## Kitty Keyboard Protocol
//!
//! Kitty supports an extended keyboard protocol that can disambiguate keys that traditional
//! terminals cannot. However, the application must opt-in to this protocol. When testing
//! applications that use the kitty keyboard protocol, you may get different results than
//! applications using legacy encoding.
//!
//! The harness defaults to kitty keyboard encoding with no flags enabled, which provides
//! a middle ground of compatibility.

use termwiz::input::{KeyCode, Modifiers};

use crate::KeyPress;

/// Common key sequences that are useful for testing.
pub mod common {
    use super::*;

    /// Ctrl+J - often equivalent to Ctrl+Enter in terminals.
    /// Use this when testing applications that execute commands on Ctrl+Enter.
    pub const CTRL_J: KeyPress = KeyPress {
        key: KeyCode::Char('j'),
        mods: Modifiers::CTRL,
    };

    /// Ctrl+M - Carriage Return, same as Enter in most contexts.
    pub const CTRL_M: KeyPress = KeyPress {
        key: KeyCode::Char('m'),
        mods: Modifiers::CTRL,
    };

    /// Ctrl+C - Interrupt signal.
    pub const CTRL_C: KeyPress = KeyPress {
        key: KeyCode::Char('c'),
        mods: Modifiers::CTRL,
    };

    /// Ctrl+D - EOF / logout.
    pub const CTRL_D: KeyPress = KeyPress {
        key: KeyCode::Char('d'),
        mods: Modifiers::CTRL,
    };

    /// Ctrl+Z - Suspend.
    pub const CTRL_Z: KeyPress = KeyPress {
        key: KeyCode::Char('z'),
        mods: Modifiers::CTRL,
    };

    /// Escape key.
    pub const ESCAPE: KeyPress = KeyPress {
        key: KeyCode::Escape,
        mods: Modifiers::NONE,
    };

    /// Enter key.
    pub const ENTER: KeyPress = KeyPress {
        key: KeyCode::Enter,
        mods: Modifiers::NONE,
    };

    /// Tab key.
    pub const TAB: KeyPress = KeyPress {
        key: KeyCode::Tab,
        mods: Modifiers::NONE,
    };

    /// Shift+Tab (backtab).
    pub const SHIFT_TAB: KeyPress = KeyPress {
        key: KeyCode::Tab,
        mods: Modifiers::SHIFT,
    };
}

/// Type a string character by character.
///
/// This is useful when you need to type text that might contain special characters,
/// or when you want to simulate actual typing rather than pasting.
///
/// # Example
/// ```ignore
/// type_string(kitty, "hello world");
/// ```
pub fn type_string(kitty: &crate::KittyHarness, text: &str) {
    for ch in text.chars() {
        kitty.send_text(&ch.to_string());
    }
}

/// Type a command string and execute it with Ctrl+J.
///
/// This is a convenience for the common pattern of typing a command and executing it,
/// particularly useful for editors that use scratch buffers for command input.
///
/// # Example
/// ```ignore
/// // Type ":write" and execute
/// type_and_execute(kitty, "write");
/// ```
pub fn type_and_execute(kitty: &crate::KittyHarness, text: &str) {
    type_string(kitty, text);
    crate::send_keys(kitty, &[common::CTRL_J]);
}
