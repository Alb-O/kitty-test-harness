# kitty-test-harness

Integration test harness for driving kitty terminal via remote control protocol and capturing rendered screen output. The library enables automated testing of terminal-based applications by launching kitty instances, sending input sequences, and extracting screen content with or without ANSI escape sequences.

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

## Terminal Key Encoding Quirks

When testing terminal applications, key encoding can be tricky. Here are common issues you may encounter:

### Ctrl+Enter vs Ctrl+J

Many terminals translate `Ctrl+Enter` to `Ctrl+J` (ASCII 0x0A). When testing applications that need `Ctrl+Enter`, use `Ctrl+J` instead:

```rust
use kitty_test_harness::{kitty_send_keys, keys};
use termwiz::input::{KeyCode, Modifiers};

// This might not work as expected through the terminal:
// kitty_send_keys!(kitty, (KeyCode::Enter, Modifiers::CTRL));

// Use Ctrl+J instead (same byte value, more reliable):
kitty_send_keys!(kitty, (KeyCode::Char('j'), Modifiers::CTRL));

// Or use the pre-defined constant:
use kitty_test_harness::send_keys;
send_keys(kitty, &[keys::CTRL_J]);
```

### Typing and Executing Commands

For editors that use scratch buffers or command modes (like Kakoune-style editors), use the `type_and_execute` helper:

```rust
use kitty_test_harness::{type_string, type_and_execute, kitty_send_keys};
use termwiz::input::KeyCode;

// Type ':' to enter command mode, type command, execute with Ctrl+J
kitty_send_keys!(kitty, KeyCode::Char(':'));
type_and_execute(kitty, "my-command arg1 arg2");
```

## Testing External Command Invocations

When testing that your application correctly invokes external commands (like `kitty @` for remote control), use mock executables:

```rust
use kitty_test_harness::{create_mock_executable, create_env_wrapper, parse_mock_log, wait_for_file};
use std::path::PathBuf;

let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
let tmp_dir = workspace.join("tmp");
let log_path = tmp_dir.join("kitty-mock.log");

// Create a mock that logs its arguments
let mock = create_mock_executable(&log_path, &tmp_dir);

// Create a wrapper that sets KITTY_REMOTE_BIN to point to our mock
let wrapper = create_env_wrapper(
    &[("KITTY_REMOTE_BIN", mock.to_str().unwrap())],
    "/path/to/your/app",
    &tmp_dir
);

// Use wrapper as the command for kitty
with_kitty_capture(&workspace, &wrapper.display().to_string(), |kitty| {
    // Trigger the action that should invoke kitty @...
    kitty_send_keys!(kitty, /* ... */);
});

// Check mock was invoked correctly
assert!(wait_for_file(&log_path, 10), "mock was not invoked");
let args = parse_mock_log(&log_path).unwrap();
assert!(args.iter().any(|a| a == "--cwd"));
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

## Development checks

Run the same checks used in CI:

```bash
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

Run kitty-backed integration tests in a GUI session:

```bash
KITTY_TESTS=1 cargo test --test kitty_smoke
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

Returns this crate's `CARGO_MANIFEST_DIR` (useful for harness-owned tests; consumers should use their own manifest dir when embedding the harness).

### `kitty_snapshot_test!`

Macro wrapper for insta snapshot tests with automatic working directory binding.

### `require_kitty()`

Boolean gate for kitty-driven tests. Checks `KITTY_TESTS`, ensures a DISPLAY/WAYLAND_DISPLAY is present, and verifies the kitty binary is on PATH; prints a skip reason and returns `false` when unavailable.

### `wait_for_clean_contains()`

Convenience helper that polls `screen_text_clean` until the cleaned text includes a substring, returning the cleaned text.

### `wait_for_screen_text_or_timeout()` and `wait_for_screen_text_clean_or_timeout()`

Timeout-explicit variants of the wait helpers that return `Result<_, WaitTimeout>` instead of silently returning the last capture on timeout. `WaitTimeout` includes elapsed time and the last captured screen sample(s).

### Key Helpers (`utils::keys`)

Pre-defined key constants for common operations:
- `keys::CTRL_J` - Ctrl+J (often equivalent to Ctrl+Enter)
- `keys::CTRL_C`, `keys::CTRL_D`, `keys::CTRL_Z` - Common control keys
- `keys::ESCAPE`, `keys::ENTER`, `keys::TAB`, `keys::SHIFT_TAB`

Helper functions:
- `type_string(kitty, text)` - Type a string character by character
- `type_and_execute(kitty, text)` - Type text and execute with Ctrl+J

### Pattern Helpers (`utils::patterns`)

- `create_mock_executable(log_path, output_dir)` - Create a script that logs invocations
- `create_env_wrapper(env_vars, target_cmd, output_dir)` - Create a wrapper that sets env vars
- `parse_mock_log(log_path)` - Parse a mock log into argument lines
- `wait_for_file(path, retries)` - Wait for a file to exist
