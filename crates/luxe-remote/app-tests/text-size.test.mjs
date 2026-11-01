// The text size: what this device kept, and setting it.

import assert from "node:assert/strict";
import { test } from "node:test";
import { setTextSize, storedSize } from "../app/text-size.js";

/** A store holding `kept` under the size's key, as localStorage would. @param {string | null} kept */
function storeWith(kept) {
  /** @type {Record<string, string>} */
  const items = kept === null ? {} : { "luxe.textSize": kept };
  return { items, getItem: (/** @type {string} */ key) => items[key] ?? null, setItem: (/** @type {string} */ key, /** @type {string} */ value) => void (items[key] = value) };
}

// The break this catches: a first visit, or a size from an older app this one doesn't offer, set
// as nothing, which leaves the page at no size at all rather than the default
test("a device that kept no size, or one not offered, gets medium", () => {
  assert.equal(storedSize(storeWith(null)), "medium");
  assert.equal(storedSize(storeWith("huge")), "medium");
  assert.equal(storedSize(storeWith("large")), "large");
});

// The break this catches: a pick that sets the page but isn't kept, so the next visit is back at
// the default, or one kept but not set until the app is opened again
test("picking a size sets the page and keeps it", () => {
  const page = { dataset: /** @type {DOMStringMap} */ ({}) };
  const store = storeWith(null);
  setTextSize("small", /** @type {any} */ (page), store);
  assert.equal(page.dataset.textSize, "small");
  assert.equal(storedSize(store), "small");
});

// The break this catches: a store that refuses (private browsing can) throwing out of the pick,
// so the size the reader chose doesn't take even for this visit
test("a store that refuses still leaves the size set", () => {
  const page = { dataset: /** @type {DOMStringMap} */ ({}) };
  const refusing = { setItem: () => { throw new Error("quota"); } };
  const warn = console.warn;
  console.warn = () => {};
  try {
    setTextSize("large", /** @type {any} */ (page), refusing);
  } finally {
    console.warn = warn;
  }
  assert.equal(page.dataset.textSize, "large");
});
