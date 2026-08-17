# Bug Report: Console Raw Input Double Key Event Signals

## Description
In standard Windows console raw mode under the XCX compiler runtime, invoking the non-blocking `input.key()` or blocking `input.key() @wait` instruction maps console events using the `crossterm` Rust library.
However, the VM implementation only inspects and returns the key code, ignoring the event kind tag (`KeyEventKind`). On systems like Windows, pressing a key once generates a key-down press event and a key-up release event, putting two distinct events with the same key code into the OS queue.
Because the VM does not filter out release events, a single physical keystroke triggers two identical event strings in XCX.

## Steps to Reproduce
1. Execute the reproduction script [reproduce.xcx](file:///D:/xcx/bugs/double_input_event/reproduce.xcx):
   ```powershell
   xcx "D:\xcx\bugs\double_input_event\reproduce.xcx"
   ```
2. Press the Enter key once.

## Observed Behavior
The CLI prints "Read 1: Got event for key: 'ENTER'" and then immediately prints "Read 2: Got event for key: 'ENTER'" without pausing or waiting for a second keystroke.

## Expected Behavior
The terminal should wait for a second distinct keyboard action before printing "Read 2".
