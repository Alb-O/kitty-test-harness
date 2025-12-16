# kitty-test-harness

Integration test harness for driving kitty terminal via remote control protocol and capturing rendered screen output. The library enables automated testing of terminal-based applications by launching kitty instances, sending input sequences, and extracting screen content with or without ANSI escape sequences.

This crate was developed to support integration tests for [frz](https://github.com/Alb-O/frz).

## Core functionality

The harness provides programmatic control over kitty terminal instances through the remote control protocol. It launches background kitty panels via Unix domain sockets, sends text and encoded key sequences, and captures screen contents for assertion in integration tests.

Screen capture supports both raw output (preserving ANSI/OSC sequences) and stripped output (plain text). Key sequences are encoded using `termwiz`'s keyboard protocol implementation rather than hardcoded escape strings, providing compatibility with kitty's keyboard protocol.

Kitty terminal must be available on PATH with remote control enabled.

## Usage

```rust
use kitty_test_harness::{kitty_send_keys, with_kitty_capture};
use termwiz::input::KeyCode;
use std::path::PathBuf;

#[test]
fn terminal_application_test() {
    let working_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    with_kitty_capture(&working_dir, "my-tui-app", |kitty| {
        std::thread::sleep(std::time::Duration::from_millis(200));
        let initial = kitty.screen_text();

        kitty_send_keys!(kitty, KeyCode::DownArrow, KeyCode::Enter);
        std::thread::sleep(std::time::Duration::from_millis(100));

        let (raw, clean) = kitty.screen_text_clean();
        assert_ne!(initial, raw);
        assert!(clean.contains("expected output"));
    });
}
```

## Snapshot testing

The `kitty_snapshot_test!` macro integrates with insta for snapshot-based regression testing. Snapshots preserve complete ANSI sequences. Generate snapshots with `cargo insta test`, review with `cargo insta review`.

```rust
use kitty_test_harness::kitty_snapshot_test;

kitty_snapshot_test!(navigation_state, |dir| {
    with_kitty_capture(&dir, "my-app", |kitty| {
        kitty_send_keys!(kitty, KeyCode::Tab, KeyCode::Tab);
        std::thread::sleep(std::time::Duration::from_millis(100));
        kitty.screen_text()
    })
});
```

## Implementation details

The harness uses kitty's `+kitten panel` with `--edge=background` to be non-intrusive but still visible in the background.

Session identification uses process ID to enable concurrent test execution. Cleanup occurs via Drop implementation, sending close-window commands to spawned panels. Screen capture uses `kitty @ get-text --ansi --extent=screen` with trailing whitespace normalization.

## API

### `KittyHarness`

Primary interface for terminal control. `launch(working_dir, command)` spawns a detached kitty panel, `send_text(text)` transmits raw strings, `screen_text()` captures current display contents, and `screen_text_clean()` returns both raw and ANSI-stripped variants.

### `with_kitty_capture`

Convenience function that launches kitty, executes a driver closure with the harness, and ensures cleanup. Generic over return type to support both test assertions and snapshot generation.

### `kitty_send_keys!`

Macro accepting KeyCode values or (KeyCode, Modifiers) tuples. Encodes key presses using termwiz and transmits to the active terminal.

### `manifest_dir()`

Returns the CARGO_MANIFEST_DIR path for resolving test-relative paths.

### `kitty_snapshot_test!`

Macro wrapper for insta snapshot tests with automatic working directory binding.
