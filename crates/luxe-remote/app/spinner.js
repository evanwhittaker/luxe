// @ts-check
// The TUI's spinner: its ten-frame braille sweep, for work in flight. The frames are stacked one per
// line in a window one cell tall, and style.css steps them past it, so the motion costs no render
// and runs on in a page busy with other work. The app's own font carries all ten glyphs.

import { html } from "./html.js";

/** luxe-tui's `SPINNER_BRAILLE`, in its order. */
export const SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/** The spinner, in the color of whatever holds it; hidden from a screen reader, beside words that say what's working. */
export function Spinner() {
  return html`<span class="spinner" aria-hidden="true"><span class="spinner-frames">${SPINNER_FRAMES.join("\n")}</span></span>`;
}
