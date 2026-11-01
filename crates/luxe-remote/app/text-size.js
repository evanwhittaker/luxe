// @ts-check
// How large this device sets the app's text: a choice kept on the device, since a phone and an iPad
// held at different distances want different sizes. The page's `data-text-size` carries it, and
// style.css turns that into the size, so nothing here writes a style.

import { signal } from "@preact/signals";
import { html } from "./html.js";

/** @typedef {"small" | "medium" | "large"} TextSize */

/** The sizes offered, smallest first. */
export const TEXT_SIZES = /** @type {TextSize[]} */ (["small", "medium", "large"]);

const DEFAULT_SIZE = /** @type {TextSize} */ ("medium");
const STORAGE_KEY = "luxe.textSize";

/** The size kept on this device, or the default when none was kept. @param {Pick<Storage, "getItem">} storage */
export function storedSize(storage) {
  const kept = storage.getItem(STORAGE_KEY);
  return TEXT_SIZES.find((size) => size === kept) ?? DEFAULT_SIZE;
}

/**
 * Sets the page to `size` and keeps it for next time. A store that refuses (private browsing can)
 * still leaves the size set for this visit.
 * @param {TextSize} size @param {HTMLElement} page @param {Pick<Storage, "setItem">} storage
 */
export function setTextSize(size, page, storage) {
  page.dataset.textSize = size;
  try {
    storage.setItem(STORAGE_KEY, size);
  } catch (error) {
    console.warn("couldn't keep the text size", error);
  }
}

/** The size showing, for the choices to mark. */
const current = signal(DEFAULT_SIZE);

/** Sets the page to the size this device kept, before the first paint. */
export function applyStoredSize() {
  current.value = storedSize(localStorage);
  document.documentElement.dataset.textSize = current.value;
}

/** The details sheet's section for what this device keeps for itself. */
export function DeviceSection() {
  return html`<section><h2 class="section-head"><span class="badge recessed">This device</span></h2>
    <h3>Text size</h3><${SizeOptions} />
  </section>`;
}

/** The session list's text size, under its notifications. */
export function TextSizeSetting() {
  return html`<section class="text-size"><h2>Text size</h2><${SizeOptions} /></section>`;
}

/** The sizes as a row of choices, the current one marked. */
function SizeOptions() {
  /** @param {TextSize} size */
  const pick = (size) => {
    current.value = size;
    setTextSize(size, document.documentElement, localStorage);
  };
  return html`<div class="options" role="group" aria-label="Text size">
    ${TEXT_SIZES.map((size) => html`<button type="button" key=${size} aria-pressed=${size === current.value}
      onClick=${() => pick(size)}>${size}</button>`)}
  </div>`;
}
