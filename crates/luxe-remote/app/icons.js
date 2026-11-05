// @ts-check
// The few drawn icons the app uses, as inline SVG in the text's color. Drawn rather than taken from
// a font, so there's no glyph a phone can lack; each button carrying one says its words to a
// screen reader.

import { html } from "./html.js";

/** @typedef {"send" | "stop" | "attach" | "close" | "dock" | "menu" | "back" | "model" | "thinking" | "github" | "review"} IconName */

const STROKE = { fill: "none", stroke: "currentColor", "stroke-width": "2.2", "stroke-linecap": "round", "stroke-linejoin": "round" };

/** @type {Record<IconName, () => unknown>} */
const DRAWINGS = {
  send: () => html`<path d="M12 19V5M5.5 11.5 12 5l6.5 6.5" ...${STROKE} />`,
  stop: () => html`<rect x="7" y="7" width="10" height="10" rx="1.5" fill="currentColor" />`,
  // a paperclip, tilted as one lies on a desk: attaching reads quicker than a picture of a photo
  attach: () => html`<path d="M17 8v8a5 5 0 0 1-10 0V6a3.25 3.25 0 0 1 6.5 0v9.5a1.5 1.5 0 0 1-3 0V8"
    transform="rotate(40 12 12)" ...${STROKE} />`,
  close: () => html`<path d="M6.5 6.5l11 11M17.5 6.5l-11 11" ...${STROKE} />`,
  dock: () => html`<rect x="3.5" y="4.5" width="17" height="15" rx="2" ...${STROKE} /><path d="M9.5 4.5v15" ...${STROKE} />`,
  menu: () => html`<path d="M4.5 7h15M4.5 12h15M4.5 17h15" ...${STROKE} />`,
  back: () => html`<path d="M15 5l-7 7 7 7" ...${STROKE} />`,
  // a robot's head, as the TUI marks the model; the composer shows the font's own mark once the
  // host gives it, and this until then
  model: () => html`<rect x="5" y="8" width="14" height="11" rx="2.5" ...${STROKE} />
    <path d="M12 8V4.5M2.5 12.5v2.5M21.5 12.5v2.5M9.5 16h5" ...${STROKE} /><circle cx="9.5" cy="12.5" r="1" fill="currentColor" />
    <circle cx="14.5" cy="12.5" r="1" fill="currentColor" />`,
  // a bulb: how hard it thinks
  thinking: () => html`<path d="M9 17.5h6M10 21h4M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.2 1 2V17.5h5V15.9c0-.8.4-1.5 1-2A6 6 0 0 0 12 3z" ...${STROKE} />`,
  // a pull request: a branch leaving its base and coming back, as GitHub draws one
  github: () => html`<circle cx="6" cy="5.5" r="2.5" ...${STROKE} /><circle cx="6" cy="18.5" r="2.5" ...${STROKE} />
    <circle cx="18" cy="18.5" r="2.5" ...${STROKE} /><path d="M6 8v8M18 16V10a3 3 0 0 0-3-3h-4M13 4.5 10.5 7l2.5 2.5" ...${STROKE} />`,
  // an outline, the desk's guide of chapters
  review: () => html`<path d="M10 6.5h10M10 12h10M10 17.5h10M4 6.5h1.5M4 12h1.5M4 17.5h1.5" ...${STROKE} />`,
};

/** One icon, sized to the line it sits in. @param {{ name: IconName }} props */
export function Icon({ name }) {
  return html`<svg class="icon" viewBox="0 0 24 24" aria-hidden="true">${DRAWINGS[name]()}</svg>`;
}
