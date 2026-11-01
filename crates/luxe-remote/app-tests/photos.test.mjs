// Photos in the composer: their size, and the frame that carries them.

import assert from "node:assert/strict";
import { test } from "node:test";
import { messageFrame } from "../app/pending.js";
import { LONGEST_EDGE, dataOf, fitWithin, forTheWire } from "../app/photos.js";

// The break this catches: a photo sent at full size (12 megapixels, megabytes over the phone's
// network), squashed out of shape, or a small screenshot blown up past its own pixels
test("a photo fits the longest edge, keeps its shape, and is never enlarged", () => {
  assert.deepEqual(fitWithin(4032, 3024), { width: LONGEST_EDGE, height: 1176 });
  assert.deepEqual(fitWithin(3024, 4032), { width: 1176, height: LONGEST_EDGE });
  assert.deepEqual(fitWithin(800, 600), { width: 800, height: 600 });
});

// The break this catches: the preview sent along, doubling every photo's size, or the data URL's
// header sent as part of the base64, which no provider can decode
test("on the wire a photo is its base64, its type and its name", () => {
  const preview = "data:image/jpeg;base64,QUJD";
  assert.equal(dataOf(preview), "QUJD");
  const photo = { data: "QUJD", mime: "image/jpeg", name: "IMG_1.HEIC", preview };
  assert.deepEqual(forTheWire([photo]), [{ data: "QUJD", mime: "image/jpeg", name: "IMG_1.HEIC" }]);
});

// The break this catches: photos held back for a turn of their own while words steer the running
// one, or steered without the photos, which the model then never sees
test("photos steer a running turn with the words, and start one otherwise", () => {
  const images = [{ data: "QUJD", mime: "image/jpeg", name: null }];
  const content = { text: "what's this?", images };
  assert.deepEqual(messageFrame("what's this?", true, images), { kind: "steer_turn", content });
  assert.deepEqual(messageFrame("what's this?", false, images), { kind: "submit_user_message", content });
});
