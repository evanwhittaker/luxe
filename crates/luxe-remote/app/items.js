// @ts-check
// The transcript's items, drawn the way the TUI draws them: your messages on a band, replies as
// text, reasoning muted, and a line for each tool call that opens to show what it returned.
//
// Text arrives as the host renders it (`render.js`). Until it does, an item shows its plain text,
// so nothing waits on the round trip to be readable.

import { useRef, useState } from "preact/hooks";
import { Answers, questionOutcome } from "./answers.js";
import { html, landedIn } from "./html.js";
import { useRendered } from "./render.js";
import { Spinner } from "./spinner.js";

/** @import { Signal } from "@preact/signals" */
/** @import { Card, Item as TranscriptItem, TextItem } from "./transcript.js" */

/** How long a copy control says it copied. */
const COPIED_MS = 1500;

/**
 * How many items are drawn at first, and how many more each tap on "Show earlier" adds. WebKit
 * slows every keystroke as a page passes a few thousand items (a 9,000-item session took 100ms a
 * key), so a long session's history waits above until it's asked for.
 */
export const WINDOW = 300;

/**
 * The items to draw when the latest `shown` are wanted, and how many earlier ones wait above.
 * @template T @param {T[]} items @param {number} shown
 */
export function windowOf(items, shown) {
  const hidden = Math.max(0, items.length - shown);
  return { hidden, drawn: hidden ? items.slice(hidden) : items };
}

/**
 * Where the drawn items start, of `length`, when at least the latest `shown` are wanted. A reader
 * following the bottom gets the latest `shown`. One reading above it keeps the start where it was,
 * so new items grow the list below them rather than taking items from over them: WebKit doesn't
 * anchor the scroll, and every item dropped from the top moved what they were reading.
 * @param {number | null} start where the drawn items started last time, if they have
 * @param {number} length @param {number} shown @param {boolean} following
 */
export function windowStart(start, length, shown, following) {
  const latest = Math.max(0, length - shown);
  return following || start === null ? latest : Math.min(start, latest);
}

/** @param {{ items: TranscriptItem[], following: boolean, waiting: boolean }} props `waiting`: on a prompt you answer */
export function Transcript({ items, following, waiting }) {
  const [shown, setShown] = useState(WINDOW);
  const start = useRef(/** @type {number | null} */ (null));
  start.current = windowStart(start.current, items.length, shown, following);
  const { hidden, drawn } = windowOf(items, items.length - start.current);
  // what's added goes in above the item at the top, and anchor.js holds that item still
  const more = () => setShown(items.length - Math.max(0, hidden - WINDOW));
  const count = html`<span class="dim">${hidden.toLocaleString()} more</span>`;
  return html`<div class="transcript" onClick=${copyCode}>
    ${hidden ? html`<button type="button" class="earlier" onClick=${more}>Show earlier ${count}</button>` : null}
    ${drawn.map((item) => html`<${Item} key=${item.key} item=${item} waiting=${waiting} />`)}
  </div>`;
}

/** @param {{ item: TranscriptItem, waiting: boolean }} props */
function Item({ item, waiting }) {
  switch (item.kind) {
    case "user": return html`<${UserMessage} item=${item} />`;
    case "assistant": return html`<${Reply} item=${item} />`;
    case "thinking": return html`<${Thinking} item=${item} />`;
    case "tool": return html`<${ToolCard} card=${item.card} waiting=${waiting} />`;
    case "error": return html`<div class="item problem">${item.text}</div>`;
    default: return html`<div class="item notice">${item.text}</div>`;
  }
}

/**
 * Host HTML when it has come, the plain text until then.
 * @param {{ className: string, rendered: string | undefined, text: string }} props
 */
function Rendered({ className, rendered, text }) {
  if (rendered === undefined) return html`<div class=${`${className} plain`}>${text}</div>`;
  return html`<div class=${className} dangerouslySetInnerHTML=${{ __html: rendered }} />`;
}

/**
 * Your message as the host renders it, then the photos it carried, which the host only counts.
 * @param {{ item: TextItem }} props
 */
function UserMessage({ item }) {
  const rendered = useRendered({ kind: "user", content: item.content ?? [] }, false);
  const message = html`<${Rendered} className="item user-message" rendered=${rendered} text=${item.text.value} />`;
  if (!item.photos?.length) return message;
  return html`${message}<div class="sent-photos">
    ${item.photos.map((photo, index) => html`<img key=${index} src=${photo} alt=${`photo ${index + 1}`} />`)}
  </div>`;
}

/** @param {{ item: TextItem }} props */
function Reply({ item }) {
  const rendered = useRendered({ kind: "answer", text: item.text.value }, !!item.provisional);
  return html`<${Rendered} className="item reply md" rendered=${rendered} text=${item.text.value} />`;
}

/** Reasoning in full, as the TUI shows it by default. @param {{ item: TextItem }} props */
function Thinking({ item }) {
  const rendered = useRendered({ kind: "reasoning", text: item.text.value }, !!item.provisional);
  return html`<div class="item thinking">
    <${Rendered} className="md reasoning" rendered=${rendered} text=${item.text.value} />
  </div>`;
}

/** @param {{ card: Signal<Card>, waiting: boolean }} props */
function ToolCard({ card, waiting }) {
  const call = card.value;
  const output = call.output_preview;
  const head = html`${spins(call, waiting) ? html`<${Spinner} />` : html`<${ToolGlyph} name=${call.name} />`}
    <span class="tool-name">${call.name}</span>
    <span class="tool-label">${commandInHeader(call) ? html`<${ToolCode} call=${call} text=${call.label} />` : call.label}</span>
    <span class="tool-time">${standing(call)}</span>
    ${body(call)}`;
  const outcome = call.name === "question" && output ? questionOutcome(output) : null;
  const className = `item tool ${call.state}${call.name === "skill" ? " skill" : ""}`;
  const pictures = html`<${ToolPictures} call=${call} />`;
  if (outcome) return html`<div class=${className}><div class="tool-head">${head}</div><${Answers} outcome=${outcome} /></div>`;
  if (!output) return html`<div class=${className}><div class="tool-head">${head}</div>${pictures}</div>`;
  return html`<div class=${className}><details>
    <summary class="tool-head">${head}</summary>
    <pre class="tool-output">${output}</pre>
  </details>${pictures}</div>`;
}

/**
 * The images a call's result carried, such as a screenshot or an image file read, shown under its
 * card in view rather than folded with its text, as the TUI draws them under the card.
 * @param {{ call: Card }} props
 */
function ToolPictures({ call }) {
  if (!call.pictures?.length) return null;
  return html`<div class="tool-pictures">
    ${call.pictures.map((picture, index) => html`<img key=${index} src=${picture} alt=${`${call.label || call.name}, image ${index + 1}`} />`)}
  </div>`;
}

/**
 * Whether the card spins, as the TUI's does while the call is being written or runs. While the turn
 * waits on you to answer a question or allow a call, a running card keeps its own mark in the same
 * color: a spinner there says the agent is busy, and only hurries you.
 * @param {Card} call @param {boolean} waiting
 */
export function spins(call, waiting) {
  return call.state === "composing" || (call.state === "running" && !waiting);
}

/** The mark the TUI's card wears for this tool, which the host knows. @param {{ name: string }} props */
function ToolGlyph({ name }) {
  const glyph = useRendered({ kind: "tool", name }, false);
  if (!glyph) return null;
  return html`<span class="tool-glyph" aria-hidden="true" dangerouslySetInnerHTML=${{ __html: glyph }} />`;
}

/**
 * Whether the header's label is the command the call ran, as the TUI's card has it: a `bash` call
 * with no body, so no description to stand in its place.
 * @param {Card} call
 */
export function commandInHeader(call) {
  return call.name === "bash" && !call.card_body?.text;
}

/** @param {Card} call */
function body(call) {
  const card = call.card_body;
  if (!card?.text) return null;
  const lines = card.text.split("\n");
  const shown = lines.slice(0, Math.max(1, card.collapsed_lines || 3));
  const more = lines.length - shown.length;
  return html`<pre class="tool-body"><${ToolCode} call=${call} text=${shown.join("\n")} />${more > 0 ? html`<span class="dim">\n… ${more} more lines</span>` : null}</pre>`;
}

/**
 * Code a card shows (a command, a written file), highlighted by the host as the TUI's card is, and
 * plain until the host answers. While the call is still being written it grows, so it isn't kept.
 * @param {{ call: Card, text: string }} props
 */
function ToolCode({ call, text }) {
  const piece = { kind: /** @type {const} */ ("tool_code"), tool: call.name, label: call.label, text };
  const rendered = useRendered(piece, call.state === "composing");
  if (rendered === undefined) return html`${text}`;
  return html`<span class="tool-code" dangerouslySetInnerHTML=${{ __html: rendered }} />`;
}

/**
 * A fence's copy control, for every rendered block at once; the page's policy allows no handler
 * in the host's HTML, so the transcript listens for it.
 * @param {Event} event
 */
function copyCode(event) {
  const button = landedIn(event, ".fence .copy");
  if (!button) return;
  event.stopPropagation();
  const code = button.closest(".fence")?.querySelector("code")?.textContent ?? "";
  navigator.clipboard.writeText(code).then(
    () => flash(button, "copied"),
    () => flash(button, "copy-failed"),
  );
}

/** @param {Element} element @param {string} className */
function flash(element, className) {
  element.classList.add(className);
  setTimeout(() => element.classList.remove(className), COPIED_MS);
}

/** How the call stands, in words: its time once done, otherwise its state. @param {Card} call */
export function standing(call) {
  if (call.state === "failed") return call.summary || "failed";
  if (call.state !== "succeeded") return call.state;
  return call.elapsed_ms == null ? "" : elapsed(call.elapsed_ms);
}

/** A duration the way the TUI writes it. @param {number} ms */
export function elapsed(ms) {
  if (ms < 1) return "<1ms";
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
}
