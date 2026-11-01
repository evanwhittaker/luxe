// @ts-check
// The tray over the composer: what this device steered that hasn't reached the agent yet, one row
// each as the TUI's tray has it, `→ steering · what you wrote`. It isn't in the transcript, where
// it would land mid-stream and scroll away under the turn's output while you wait on it; it lands
// there when a checkpoint delivers it. Withdraw takes it all back into the composer to say better.

import { html } from "./html.js";

/** @import { Steer } from "./pending.js" */
/** @import { Draft } from "./picker.js" */

/** The TUI's steering mark (`Icon::Steering`), which the app's font draws. */
const STEERING_GLYPH = "\u{f0054}";

/**
 * What a held steer's row says: its words on one line, or what it carries when it's photos alone.
 * @param {Steer} steer
 */
export function trayText(steer) {
  const words = steer.text.split(/\s+/).filter(Boolean).join(" ");
  if (words || !steer.photos.length) return words;
  return steer.photos.length === 1 ? "1 photo" : `${steer.photos.length} photos`;
}

/**
 * The draft with taken-back steers ahead of what's being written, in the order they were sent, as
 * the TUI's recall puts them; the caret goes at the end.
 * @param {Steer[]} held @param {Draft} draft @returns {Draft}
 */
export function restoredDraft(held, draft) {
  const parts = [...held.map((steer) => steer.text.trim()), draft.text.trim()].filter(Boolean);
  const text = parts.join("\n\n");
  return { text, caret: text.length, placed: true };
}

/** @param {{ held: Steer[], withdraw: () => void }} props */
export function SteeringTray({ held, withdraw }) {
  if (!held.length) return null;
  return html`<div class="tray">
    <ul aria-label="Waiting to steer">${held.map((steer, index) => html`<li key=${index}>
      <span class="tray-mark" aria-hidden="true">${STEERING_GLYPH}</span> steering · ${trayText(steer)}
      ${steer.photos.map((photo) => html`<img src=${photo.preview} alt=${photo.name ?? "photo"} />`)}
    </li>`)}</ul>
    <button type="button" class="withdraw" onClick=${withdraw}>withdraw</button>
  </div>`;
}
