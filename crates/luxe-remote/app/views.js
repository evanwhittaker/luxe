// @ts-check
// The full-screen views over the session, as the TUI opens `/github` and Guided Review over its
// whole screen: one at a time, covering the sidebar too, closed by Escape or the hint line's
// close. Each draws its body; this draws what they share: the TUI's rounded frame titled in its
// top border (GitHub's; the desk has none), and the hint line along the foot, each hint a button.
// A hint shows its key on a wide screen, where a keyboard is likely, and its word everywhere.

import { signal } from "@preact/signals";
import { useLayoutEffect, useRef } from "preact/hooks";
import { html } from "./html.js";
import { useMedia } from "./layout.js";
import { ChromeMark } from "./marks.js";

/**
 * @typedef {"github" | "review"} ViewName
 * @typedef {{ key: string, word: string, run: () => void, tone?: "accent" | "warning" }} Hint
 * @typedef {ReturnType<typeof createViews>} Views
 */

/**
 * Wide enough for a view's two panes side by side: the TUI splits `/github` at 124 cells and the
 * desk near it, about 64rem at the default text size, so an iPad on its side splits and one
 * standing up shows one pane at a time. views.css uses the same query.
 */
export const SPLIT = "(min-width: 64rem) and (min-height: 30rem)";

/** Whether a view's two panes sit side by side. */
export const useSplit = () => useMedia(SPLIT);

/** No view open. */
export function createViews() {
  const shown = signal(/** @type {ViewName | null} */ (null));
  return { shown, open: (/** @type {ViewName} */ name) => (shown.value = name), close: () => (shown.value = null) };
}

/**
 * The frame a view sits in, titled as the TUI titles it, with `hints` along the foot and `place`
 * (where the cursor is, `1/2`) at its end.
 * @param {{ name: ViewName, label: string, title?: string, hints: Hint[], place?: string, children: any }} props
 */
export function ViewFrame({ name, label, title, hints, place, children }) {
  return html`<div class=${`view view-${name}`} role="dialog" aria-modal="true" aria-label=${label}>
    <div class=${title ? "view-frame framed" : "view-frame"}>
      ${title ? html`<p class="view-title"><${ChromeMark} mark=${name} drawn=${name} /> ${title}</p>` : null}
      <div class="view-body">${children}</div>
      <${HintLine} hints=${hints} place=${place} />
    </div>
  </div>`;
}

/**
 * Host-drawn `markup` with the one element whose `attribute` is `current` marked as the cursor's,
 * as the TUI puts its cursor row on a slab, and scrolled into sight when it moves.
 * @param {{ markup: string, attribute: string, current: string | null, className: string, onClick: (event: MouseEvent) => void }} props
 */
export function CursorPiece({ markup, attribute, current, className, onClick }) {
  const ref = useRef(/** @type {HTMLDivElement | null} */ (null));
  useLayoutEffect(() => {
    const box = ref.current;
    if (!box) return;
    for (const element of box.querySelectorAll("[aria-current]")) element.removeAttribute("aria-current");
    const found = current ? box.querySelector(`[${attribute}="${CSS.escape(current)}"]`) : null;
    found?.setAttribute("aria-current", "true");
    found?.scrollIntoView?.({ block: "nearest" });
  }, [markup, current]);
  return html`<div ref=${ref} class=${className} onClick=${onClick} dangerouslySetInnerHTML=${{ __html: markup }}></div>`;
}

/** The TUI's hint line: each hint its key in the accent then its word, split by dim dots. @param {{ hints: Hint[], place?: string }} props */
function HintLine({ hints, place }) {
  return html`<nav class="hints" aria-label="Actions">
    ${hints.map((hint, index) => html`${index ? html`<span class="hint-dot" aria-hidden="true">·</span>` : null}
      <button type="button" key=${hint.word} class=${`hint${hint.tone ? ` hint-${hint.tone}` : ""}`} onClick=${hint.run}>
        <kbd class="hint-key">${hint.key}</kbd> <span class="hint-word">${hint.word}</span></button>`)}
    ${place ? html`<span class="hint-place">${place}</span>` : null}
  </nav>`;
}

/**
 * Runs the hint whose key was pressed, so a keyboard drives the view as it drives the TUI. Keys
 * typed into a field are the field's; Escape is left to `useEscape`, which knows what's on top.
 * @param {Hint[]} hints @param {Record<string, () => void>} [more] keys no hint shows
 */
export function useViewKeys(hints, more = {}) {
  useLayoutEffect(() => {
    /** @param {KeyboardEvent} event */
    const listener = (event) => {
      if (typing(event.target) || event.metaKey || event.ctrlKey || event.altKey) return;
      const run = more[keyName(event)] ?? hints.find((hint) => hint.key === keyName(event))?.run;
      if (!run) return;
      event.preventDefault();
      run();
    };
    addEventListener("keydown", listener);
    return () => removeEventListener("keydown", listener);
  }, [hints, more]);
}

/** A key as a hint names it: `enter`, `space`, `tab`, `shift+tab`, an arrow, or the letter. @param {KeyboardEvent} event */
export function keyName({ key, shiftKey }) {
  const named = { Enter: "enter", " ": "space", Tab: "tab", ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→" };
  const name = named[/** @type {keyof typeof named} */ (key)] ?? key.toLowerCase();
  return shiftKey && name === "tab" ? "shift+tab" : name;
}

/** Whether keys go into `target`: a field or anything editable. @param {EventTarget | null} target */
function typing(target) {
  const element = /** @type {HTMLElement | null} */ (target);
  return Boolean(element?.closest?.("input, textarea, select, [contenteditable]"));
}
