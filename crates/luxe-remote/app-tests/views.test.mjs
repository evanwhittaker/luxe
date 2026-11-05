// The full-screen views: which is open, the keys that drive them, and how a session opens one.

import assert from "node:assert/strict";
import { test } from "node:test";
import { createViews, keyName } from "../app/views.js";
import { openView } from "../app/session-views.js";
import { createGithub } from "../app/github.js";
import { createReview } from "../app/review.js";

// The break this catches: a view that can't be closed, or two open at once
test("one view at a time, and close shows none", () => {
  const views = createViews();
  views.open("github");
  views.open("review");
  assert.equal(views.shown.value, "review");
  views.close();
  assert.equal(views.shown.value, null);
});

// The break this catches: a key named otherwise than its hint, so pressing it does nothing
test("keys are named as the hints name them", () => {
  const key = (/** @type {string} */ name, shiftKey = false) => keyName(/** @type {any} */ ({ key: name, shiftKey }));
  assert.equal(key("Enter"), "enter");
  assert.equal(key(" "), "space");
  assert.equal(key("Tab", true), "shift+tab");
  assert.equal(key("ArrowLeft"), "←");
  assert.equal(key("C"), "c");
});

/** A session's view hooks on a screen that `splits` or not, with what was sent. @param {boolean} splits */
function session(splits) {
  globalThis.matchMedia = /** @type {any} */ (() => ({ matches: splits }));
  /** @type {any[]} */
  const sent = [];
  const cleared = { count: 0 };
  const hooks = {
    views: createViews(), github: createGithub(), review: createReview(), mention: () => {},
    send: (/** @type {any} */ frame) => sent.push(frame) > 0, clear: () => void cleared.count++,
  };
  hooks.github.open.value = /** @type {any} */ ({ subject: { number: 1 } });
  hooks.review.open.value = "chapter:2";
  return { hooks, sent, cleared };
}

// The break this catches: Review on a pull request opening the working tree's review, or the view
// opening over the sidebar sheet left up
test("a target loads its review as the view opens", () => {
  const { hooks, sent, cleared } = session(true);
  openView(hooks, "review", { kind: "pull_request", number: 42 });
  assert.equal(hooks.views.shown.value, "review");
  assert.equal(cleared.count, 1);
  assert.deepEqual(sent.map((frame) => frame.content.target), [{ kind: "pull_request", number: 42 }]);
});

// The break this catches: a phone reopening on the row or chapter read last, skipping the list or
// guide, or a split screen losing the one beside it
test("a phone opens each view on its overview, and a split screen keeps its place", () => {
  const phone = session(false);
  openView(phone.hooks, "review");
  assert.equal(phone.hooks.review.open.value, null);
  openView(phone.hooks, "github");
  assert.equal(phone.hooks.github.open.value, null);
  const wide = session(true);
  openView(wide.hooks, "review");
  openView(wide.hooks, "github");
  assert.deepEqual([wide.hooks.review.open.value, wide.hooks.github.open.value?.subject.number], ["chapter:2", 1]);
});
