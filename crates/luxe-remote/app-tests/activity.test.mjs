// The activity bar's sweep, step for step with luxe-tui's, and the turn's elapsed time.

import assert from "node:assert/strict";
import { test } from "node:test";
import { activityCells, BAR_CELLS, elapsedCompact, shadeStyle } from "../app/activity.js";

const lit = (cells) => cells.map((cell) => (cell.ramp === "trail" ? cell.glyph : ".")).join("");

// The break this catches: the comet's length drifting from the TUI's two fifths of the bar (capped
// at the ten shades), or its tail tapering the wrong way. At frame 18 the head is 9 cells in, and
// the comet's 9 cells (two fifths of 24) trail back to cell 1.
test("the comet runs two fifths of the bar, heaviest at its head", () => {
  const cells = activityCells(18, BAR_CELLS, 10);
  assert.equal(cells.length, 24);
  assert.equal(lit(cells), ".▏▏▎▎▎▍▍▌▌" + ".".repeat(14));
  assert.equal(cells[9].step, 0);
  assert.equal(cells[1].step, 9);
});

// The break this catches: the comet wrapping round instead of bouncing, or the rail drawn while
// nothing's on it. A leg of 32 steps takes 64 frames, then 9 frames hold the rail empty, and four
// frames into the way back the head is 2 cells in from the right.
test("it holds, then sweeps back from the end it left by", () => {
  assert.equal(lit(activityCells(70, BAR_CELLS, 10)), ".".repeat(24));
  assert.ok(activityCells(70, BAR_CELLS, 10).every((cell) => cell.step === 9));
  assert.equal(lit(activityCells(77, BAR_CELLS, 10)), ".".repeat(21) + "▌▌▍");
});

// The break this catches: the rail lit evenly rather than by the comet's glow, which fades over
// half the bar to the darkest shade.
test("the rail glows nearest the comet", () => {
  const cells = activityCells(18, BAR_CELLS, 10);
  assert.ok(cells[10].step < cells[16].step);
  assert.equal(cells[23].step, 9);
});

// The break this catches: planning's comet drawn in build's shades, or a theme without the derived
// shades leaving the bar invisible.
test("each cell wears its mode's shade with a fallback", () => {
  assert.match(shadeStyle({ glyph: "▌", ramp: "trail", step: 2 }, true), /var\(--luxe-trail-plan-2, var\(--mode/);
  assert.match(shadeStyle({ glyph: "▌", ramp: "trail", step: 2 }, false), /var\(--luxe-trail-2, /);
  assert.match(shadeStyle({ glyph: "▏", ramp: "rail", step: 4 }, false), /var\(--luxe-rail-4, /);
});

// The break this catches: the time worded differently from the TUI's row, or minutes unpadded.
test("the elapsed time reads as the TUI's", () => {
  assert.equal(elapsedCompact(12_400), "12s");
  assert.equal(elapsedCompact(184_000), "3m 04s");
  assert.equal(elapsedCompact(3_720_000), "1h 02m");
  assert.equal(elapsedCompact(-5), "0s");
});
