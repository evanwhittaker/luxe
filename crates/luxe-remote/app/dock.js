// @ts-check
// The dock's sections the host draws whole in the sidebar (panel.js), which the header's button and
// `/dock` show and hide (layout.js): the plan at the top, where a long one scrolls in its own box,
// and MODEL and SESSION at the foot, as the TUI's rail has them. A changed file is opened from the
// Files tree (files-view.js), and its diff shows in the sheet here.

import { useLayoutEffect, useRef } from "preact/hooks";
import { html } from "./html.js";
import { Icon } from "./icons.js";
import { useRendered } from "./render.js";
import { PickablePiece } from "./lines-view.js";
import { planScroll } from "./workspace.js";
import { useWide } from "./layout.js";

/** @import { OpenDiff, Todos } from "./workspace.js" */
/** @typedef {{ facts: Record<string, any>, mode: string }} Telemetry */

/** The plan, left out until there is one, as the TUI's rail leaves it out. @param {{ hooks: { todos: Todos | null } }} props */
export function PlanPart({ hooks: { todos } }) {
  if (!todos?.total) return null;
  return html`<section class="part" data-part="plan" aria-label="Plan"><div class="dock-part">
    <${PlanSection} piece=${{ kind: "plan", todos }} />
  </div></section>`;
}

/** The model, its thinking level and the mode. @param {{ hooks: Telemetry }} props */
export function ModelPart({ hooks: { facts, mode } }) {
  return html`<${HostPart} id="model" title="Model" piece=${{ kind: "model", ...facts, mode }} />`;
}

/** The window, the tokens and the cost, once a turn has measured any. @param {{ hooks: Telemetry }} props */
export function SessionPart({ hooks: { facts } }) {
  return html`<${HostPart} id="session" title="Session" piece=${{ kind: "session", ...facts }} />`;
}

/**
 * A part the host draws whole, heading and all, left out while it draws nothing.
 * @param {{ id: string, title: string, piece: import("./render.js").Piece }} props
 */
function HostPart({ id, title, piece }) {
  const rendered = useRendered(piece, true);
  if (!rendered) return null;
  return html`<section class="part" data-part=${id} aria-label=${title}><div class="dock-part">
    <section class="dock-section" dangerouslySetInnerHTML=${{ __html: rendered }}></section>
  </div></section>`;
}

/** @param {{ piece: import("./render.js").Piece }} props */
function PlanSection({ piece }) {
  const rendered = useRendered(piece, true);
  const section = useRef(/** @type {HTMLElement | null} */ (null));
  const handsOn = usePlanFollow(section, rendered);
  return html`<section class="dock-section" ref=${section} onTouchStart=${handsOn} onWheel=${handsOn}
    dangerouslySetInnerHTML=${{ __html: rendered ?? "" }}></section>`;
}

/**
 * Scrolls a long plan to the work in hand each time it's drawn, as the TUI's rail does, until the
 * reader scrolls it themselves. Returns what marks that.
 * @param {{ current: HTMLElement | null }} section @param {string | null | undefined} rendered
 */
function usePlanFollow(section, rendered) {
  const moved = useRef(false);
  useLayoutEffect(() => {
    const list = section.current?.querySelector(".todos");
    if (!(list instanceof HTMLElement) || moved.current) return;
    const rows = [...list.children].map((row) => ({
      bottom: /** @type {HTMLElement} */ (row).offsetTop + /** @type {HTMLElement} */ (row).offsetHeight,
      status: row.className.match(/todo-(\S+)/)?.[1] ?? "",
    }));
    list.scrollTop = planScroll(rows, list.clientHeight);
  }, [rendered]);
  return () => (moved.current = true);
}

/** The wide header's button that shows and hides the sidebar's rail. @param {{ shown: boolean, toggle: () => void }} props */
export function DockToggle({ shown, toggle }) {
  return html`<button type="button" class=${shown ? "dock-toggle on" : "dock-toggle"}
    aria-label=${shown ? "Hide the sidebar" : "Show the sidebar"} aria-expanded=${shown}
    onClick=${toggle}><${Icon} name="dock" /></button>`;
}

/**
 * One file's diff until it's closed: over the page on a phone, beside the rail on a wide screen.
 * @param {{ open: OpenDiff | null, close: () => void, retry: () => void, ask: (words: string) => void }} props
 */
export function DiffSheet({ open, close, retry, ask }) {
  const wide = useWide();
  useEscape(open ? close : null);
  if (!open) return null;
  return html`<div class="sheet peek" role="dialog" aria-modal=${!wide} aria-label=${`Changes to ${open.path}`}>
    <header class="bar">
      <span class="title">${open.path}</span>
      <button type="button" class="close" aria-label="Close the diff" onClick=${close}><${Icon} name="close" /></button>
    </header>
    <div class="sheet-body"><${DiffBody} open=${open} retry=${retry} ask=${ask} /></div>
  </div>`;
}

/** @param {{ open: OpenDiff, retry: () => void, ask: (words: string) => void }} props */
function DiffBody({ open, retry, ask }) {
  if (open.lost) {
    return html`<p class="diff-note">Connection lost before the diff loaded.
      <button type="button" class="retry" onClick=${retry}>Try again</button></p>`;
  }
  if (open.waiting) return html`<p class="diff-note">Loading diff…</p>`;
  /** @type {import("./render.js").Piece} */
  const piece = open.diff ? { kind: "diff", diff: open.diff } : { kind: "diff_failed", error: open.error };
  return html`<${PickablePiece} piece=${piece} ask=${ask} />`;
}

/**
 * Escape closes the sheet, for an iPad with a keyboard. Armed as the sheet reaches the page rather
 * than after the next paint, so an Escape pressed straight after a pane opens or closes isn't lost.
 * @param {(() => void) | null} onEscape
 */
export function useEscape(onEscape) {
  useLayoutEffect(() => {
    if (!onEscape) return;
    /** @param {KeyboardEvent} event */
    const listener = (event) => event.key === "Escape" && onEscape();
    addEventListener("keydown", listener);
    return () => removeEventListener("keydown", listener);
  }, [onEscape]);
}
