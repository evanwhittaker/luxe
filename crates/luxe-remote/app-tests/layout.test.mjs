// Whether the dock shows and on which side: started from the page, changed by `/dock` and the
// header's button.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { SIDEBAR_PARTS, WIDE, createDock, dockAfter, dockVisible, phoneDock, startingMode } from "../app/layout.js";

// The break this catches: the plan and changes moved below the project's views, where a phone's
// one button no longer opens on them; or GitHub unfolded from the start, so every iPad session's
// rail asks GitHub for its lists unasked
test("the sidebar runs dock first, and only what's cheap to ask for starts unfolded", () => {
  assert.deepEqual(SIDEBAR_PARTS.map(({ id }) => id), ["dock", "files", "github", "review"]);
  assert.deepEqual(SIDEBAR_PARTS.filter(({ unfolded }) => unfolded).map(({ id }) => id), ["dock", "files"]);
});

// The break this catches: `/dock` on a phone doing nothing now the dock lives in the sheet, or a
// bare one that can't close what it opened
test("on a phone, /dock opens the sidebar and a bare one closes it", () => {
  assert.equal(phoneDock("toggle", false), "open");
  assert.equal(phoneDock("toggle", true), "close");
  assert.equal(phoneDock("show", true), "open");
  assert.equal(phoneDock("hide", false), "close");
  assert.equal(phoneDock("left", false), null);
  assert.equal(phoneDock("auto", true), null);
});

// The break this catches: a stylesheet's breakpoint moved without the script's, so on a screen
// between the two the button says the rail is hidden while it shows, or a peek that's only a pane
// beside the rail is announced as covering the page
test("the script's wide screen is the stylesheets'", () => {
  for (const sheet of ["peek.css"]) {
    const css = readFileSync(new URL(`../app/${sheet}`, import.meta.url), "utf8");
    assert.ok(css.includes(`@media ${WIDE} {`), `${sheet} has no @media ${WIDE}`);
  }
});

// The break this catches: the config's mode ignored, or a value the app doesn't know taken as is,
// where the TUI falls back to auto
test("the mode starts from the page, and auto when it says nothing known", () => {
  assert.equal(startingMode({ dockMode: "shown" }), "shown");
  assert.equal(startingMode({ dockMode: "hidden" }), "hidden");
  assert.equal(startingMode({ dockMode: "sideways" }), "auto");
  assert.equal(startingMode({}), "auto");
});

// The break this catches: auto showing the dock over a phone's conversation, or hiding the rail
// on an iPad
test("auto shows the dock on a wide screen only", () => {
  assert.equal(dockVisible("auto", true), true);
  assert.equal(dockVisible("auto", false), false);
});

// The break this catches: shown or hidden bent by the screen's width, which only auto follows
test("shown and hidden hold at any width", () => {
  assert.equal(dockVisible("shown", false), true);
  assert.equal(dockVisible("hidden", true), false);
});

// The break this catches: the toggle flipping the mode rather than what's on screen, so the first
// tap on an iPad in auto shows a dock already showing
test("the toggle flips what's on screen", () => {
  assert.equal(dockAfter("auto", "toggle", true), "hidden");
  assert.equal(dockAfter("auto", "toggle", false), "shown");
  assert.equal(dockAfter("hidden", "toggle", true), "shown");
});

// The break this catches: a `/dock` verb setting the wrong mode, or a side verb changing it
test("show, hide and auto set the mode, and the sides leave it", () => {
  assert.equal(dockAfter("hidden", "show", false), "shown");
  assert.equal(dockAfter("shown", "hide", false), "hidden");
  assert.equal(dockAfter("shown", "auto", false), "auto");
  assert.equal(dockAfter("shown", "left", false), null);
});

// The break this catches: the side kept anywhere the stylesheet doesn't read, or a side verb
// taken for a mode
test("a side moves the page's dock side and leaves the mode", () => {
  const page = { dataset: { dockMode: "shown", dockSide: "left" } };
  const dock = createDock(/** @type {any} */ (page), () => true);
  dock.apply("right");
  assert.equal(page.dataset.dockSide, "right");
  assert.equal(dock.mode.value, "shown");
});

// The break this catches: the screen's width read once at start, so a rotated iPad's toggle
// flips the wrong way
test("the toggle reads the width when it's tapped", () => {
  let wide = true;
  const dock = createDock(/** @type {any} */ ({ dataset: {} }), () => wide);
  wide = false;
  dock.apply("toggle");
  assert.equal(dock.mode.value, "shown");
});
