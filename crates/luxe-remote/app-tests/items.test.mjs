// How a tool card leads: with the spinner while it works, its own mark otherwise, and its command
// colored as shell when the header is where the command shows.

import assert from "node:assert/strict";
import { test } from "node:test";
import { commandInHeader, spins } from "../app/items.js";
import { SPINNER_FRAMES } from "../app/spinner.js";

const call = (name, state) => ({ call_id: "c", name, label: "", state, revision: 0 });

// The break this catches: a call at work without the spinner, whether the tool runs it or the model
// is still writing it
test("a call spins while it runs and while it's being written", () => {
  assert.ok(spins(call("bash", "running"), false));
  assert.ok(spins(call("edit", "composing"), false));
});

// The break this catches: a spinner over a card that isn't moving: one waiting its turn, landed,
// or stopped short
test("a call that isn't moving keeps its own mark", () => {
  for (const state of ["queued", "succeeded", "failed", "cancelled", "interrupted"]) {
    assert.ok(!spins(call("bash", state), false), state);
  }
});

// The break this catches: a spinner hurrying you over the question or permission the turn waits on,
// or one taken from a call still being written, which the TUI keeps spinning
test("while the turn waits on you, only a call being written spins", () => {
  assert.ok(!spins(call("edit", "running"), true));
  assert.ok(spins(call("edit", "composing"), true));
});

// The break this catches: a described command's header (its description) colored as shell, a
// one-liner's left plain, or another tool's label taken for a command
test("only a bash header with no body is the command itself", () => {
  const body = { text: "wc -l main.rs", total_lines: 1, collapsed_lines: 1 };
  assert.ok(commandInHeader({ ...call("bash", "running"), card_body: null }));
  assert.ok(!commandInHeader({ ...call("bash", "running"), card_body: body }));
  assert.ok(!commandInHeader({ ...call("read", "running"), card_body: null }));
});

// The break this catches: the sweep drifting from the TUI's, in its frames or their order
test("the spinner is the TUI's ten-frame braille sweep", () => {
  assert.equal(SPINNER_FRAMES.join(""), "\u280B\u2819\u2839\u2838\u283C\u2834\u2826\u2827\u2807\u280F");
});
