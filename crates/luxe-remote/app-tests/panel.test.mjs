// The sidebar's state: the phone's sheet open or not, and which parts are unfolded.

import assert from "node:assert/strict";
import { test } from "node:test";
import { createPanel } from "../app/panel.js";

const unfolded = (/** @type {ReturnType<typeof createPanel>} */ panel) => [...panel.unfolded.value].sort();

// The break this catches: the phone's button or `/dock` not showing the sheet, or a close that
// doesn't put it away
test("opening shows the sheet and closing hides it", () => {
  const panel = createPanel();
  panel.open();
  assert.equal(panel.shown.value, true);
  panel.close();
  assert.equal(panel.shown.value, false);
});

// The break this catches: a heading's tap only ever unfolding, so a long part can't be put away,
// or folding its neighbors with it
test("a heading's tap folds an unfolded part and unfolds a folded one", () => {
  const panel = createPanel();
  const start = unfolded(panel);
  panel.fold("files");
  assert.deepEqual(unfolded(panel), start.filter((part) => part !== "files"));
  panel.fold("files");
  assert.deepEqual(unfolded(panel), start);
});

// The break this catches: closing the phone's sheet forgetting what was folded, so Files opens
// unfolded again each time
test("closing keeps the folds", () => {
  const panel = createPanel();
  panel.fold("files");
  panel.open();
  panel.close();
  assert.equal(panel.shown.value, false);
  assert.ok(!panel.unfolded.value.has("files"));
});
