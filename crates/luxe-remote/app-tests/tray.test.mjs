// The tray over the composer: what each held steer's row says, and what taking them back gives.

import assert from "node:assert/strict";
import { test } from "node:test";
import { restoredDraft, trayText } from "../app/tray.js";

const photo = { data: "QUJD", mime: "image/jpeg", name: null, preview: "data:image/jpeg;base64,QUJD" };

// The break this catches: a steer written over several lines making a row as tall as the message,
// so a long one pushes the composer off the screen
test("a held steer's row is its words on one line", () => {
  assert.equal(trayText({ text: "check the\n\n  windows path", photos: [photo] }), "check the windows path");
});

// The break this catches: a steer of photos alone shown as an empty row, which says nothing waits
test("a steer of photos alone says how many", () => {
  assert.equal(trayText({ text: " ", photos: [photo] }), "1 photo");
  assert.equal(trayText({ text: "", photos: [photo, photo] }), "2 photos");
});

// The break this catches: taken-back steers put after what's being written, or out of the order
// they were sent, or the caret left at the start where the next word would split them
test("taken-back steers go ahead of the draft, in the order they were sent", () => {
  const held = [{ text: "first", photos: [] }, { text: "second", photos: [photo] }];
  assert.deepEqual(restoredDraft(held, { text: "still writing", caret: 3 }), {
    text: "first\n\nsecond\n\nstill writing", caret: 28, placed: true,
  });
});

// The break this catches: blank lines left where a photo-only steer or an empty draft was
test("nothing empty leaves a gap", () => {
  const held = [{ text: "", photos: [photo] }, { text: "look at this", photos: [] }];
  assert.equal(restoredDraft(held, { text: "", caret: 0 }).text, "look at this");
});
