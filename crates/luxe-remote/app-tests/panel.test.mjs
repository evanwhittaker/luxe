// The sidebar's state: the phone's sheet open or not, which parts are unfolded, and where a jump
// lands.

import assert from "node:assert/strict";
import { test } from "node:test";
import { createPanel } from "../app/panel.js";

const unfolded = (/** @type {ReturnType<typeof createPanel>} */ panel) => [...panel.unfolded.value].sort();

// The break this catches: a jump to a folded part landing on its bare heading, or not asking for
// the part to be brought into view at all
test("a jump unfolds the part it lands on and asks for it in view", () => {
  const panel = createPanel();
  panel.jump("github");
  assert.deepEqual(unfolded(panel), ["dock", "files", "github"]);
  assert.equal(panel.target.value, "github");
});

// The break this catches: the phone's button opening the sheet scrolled to some part, or `/dock`
// opening it somewhere other than the dock
test("opening shows the sheet, jumping only when told where", () => {
  const panel = createPanel();
  panel.open();
  assert.equal(panel.shown.value, true);
  assert.equal(panel.target.value, null);
  panel.open("dock");
  assert.equal(panel.target.value, "dock");
});

// The break this catches: a heading's tap only ever unfolding, so a long part can't be put away,
// or folding its neighbors with it
test("a heading's tap folds an unfolded part and unfolds a folded one", () => {
  const panel = createPanel();
  panel.fold("files");
  assert.deepEqual(unfolded(panel), ["dock"]);
  panel.fold("review");
  panel.fold("files");
  assert.deepEqual(unfolded(panel), ["dock", "files", "review"]);
});

// The break this catches: closing the phone's sheet forgetting what was unfolded, so GitHub is
// folded again, and asked for again, each time it opens
test("closing keeps the folds", () => {
  const panel = createPanel();
  panel.open("review");
  panel.close();
  assert.equal(panel.shown.value, false);
  assert.ok(panel.unfolded.value.has("review"));
});
