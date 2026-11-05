// Holding the reader's place: which item it's held on, and how far to scroll to put it back.

import assert from "node:assert/strict";
import { test } from "node:test";
import { anchorIn, anchorShift } from "../app/anchor.js";

/**
 * An element as far as the anchor reads one: its box, its classes, whether it's on the page.
 * @returns {any}
 */
const element = (top, height, classes = []) => ({
  box: { top, bottom: top + height },
  isConnected: true,
  classList: { contains: (name) => classes.includes(name) },
  getBoundingClientRect() { return this.box; },
});

// The break this catches: the place held on an item already scrolled past (so a change inside what
// you're reading moves it) instead of the first one still showing
test("the anchor is the first item still showing", () => {
  const items = [element(-300, 200), element(-100, 150), element(50, 100)];
  const anchor = anchorIn(/** @type {any} */ ({ children: items }), 0);
  assert.equal(anchor?.item, items[1]);
  assert.equal(anchor?.offset, -100);
});

// The break this catches: the place held on "Show earlier", which stays first as the history it
// adds goes in under it, so the page jumps by the whole page of history
test("the Show earlier button is never the anchor", () => {
  const items = [element(10, 40, ["earlier"]), element(60, 100)];
  assert.equal(anchorIn(/** @type {any} */ ({ children: items }), 0)?.item, items[1]);
});

// The break this catches: the scroll moved the wrong way, or by the item's offset rather than by
// how far it moved, when what's above it grew
test("the shift is how far the anchor moved", () => {
  const item = element(-100, 150);
  const anchor = { item, offset: -100 };
  item.box = { top: 140, bottom: 290 };
  assert.equal(anchorShift(anchor, 0), 240);
  item.isConnected = false;
  assert.equal(anchorShift(anchor, 0), 0, "an item gone from the page holds nothing");
});
