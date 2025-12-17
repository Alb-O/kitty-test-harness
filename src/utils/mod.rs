//! Module for utility functions and helpers for the kitty test harness.

/// Helpers for environment detection and test gating.
pub mod env;
/// Terminal key encoding helpers and common key constants.
pub mod keys;
/// Common testing patterns (mock executables, env wrappers, etc.).
pub mod patterns;
/// Helpers for waiting for certain conditions in the kitty harness.
pub mod wait;
/// Helpers for managing kitty windows and panels.
pub mod window;
