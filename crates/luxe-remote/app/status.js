// @ts-check
// The status line under the composer: the mode, which a tap switches, then the bar's right-hand
// facts (context, cost, thinking, model) as the host words them, so they read as the TUI's do. A
// phone keeps it to one row and leaves the cost to the details sheet a tap on the facts opens.

import { batch, signal } from "@preact/signals";
import { MODES } from "./details.js";
import { html } from "./html.js";
import { useRendered } from "./render.js";
import { Spinner } from "./spinner.js";

/** @import { Signal } from "@preact/signals" */
/** @import { Frame } from "./connection.js" */
/** @import { Item } from "./transcript.js" */
/** @typedef {{ facts: Signal<Record<string, any>>, mode: Signal<string> }} State */

/** Nothing reported yet, and the function that folds frames into what is. */
export function createStatus() {
  /** @type {State} */
  const state = { facts: signal({}), mode: signal("") };
  /** @param {Frame} frame */
  const fold = (frame) => batch(() => apply(state, frame));
  return { facts: state.facts, mode: state.mode, apply: fold };
}

/** @param {State} state @param {Frame} frame */
function apply(state, { kind, content }) {
  switch (kind) {
    case "state_changed":
      state.mode.value = content.mode ?? "";
      return update(state, { model: content.model, thinking: content.thinking });
    case "context_usage": return update(state, { context: content });
    case "plan_usage": return update(state, { plan: content });
    // the total is all the line shows; the breakdown would only make every request longer
    case "session_usage": return update(state, { session: { total: content.total } });
  }
}

/** @param {State} state @param {Record<string, any>} changed */
function update(state, changed) {
  state.facts.value = { ...state.facts.value, ...changed };
}

/** The frame that switches from `mode` to the next one. @param {string} mode @returns {Frame} */
export function nextModeFrame(mode) {
  const next = MODES[(MODES.indexOf(mode) + 1) % MODES.length];
  return { kind: "set_mode", content: next };
}

/**
 * What the session is doing, as the TUI's activity segment says it: "compacting" while a summary
 * is written, which can take a minute with nothing streaming, else the tool running, or "working"
 * while the model writes; null when idle. Only the turn's own items are looked through, back to the
 * message that started it, so a long session isn't walked on every render.
 * @param {boolean} busy @param {Item[]} items @param {boolean} compacting @returns {string | null}
 */
export function activityLabel(busy, items, compacting = false) {
  if (compacting) return "compacting";
  if (!busy) return null;
  for (let index = items.length - 1; index >= 0; index -= 1) {
    const item = items[index];
    if (item.kind === "user") break;
    if (item.kind === "tool" && item.card.value.state === "running") return item.card.value.name;
  }
  return "working";
}

/**
 * @param {{ facts: Record<string, any>, mode: string, activity: string | null,
 *   send: (frame: Frame) => boolean, details: () => void }} props
 */
export function StatusLine({ facts, mode, activity, send, details }) {
  const rendered = useRendered({ kind: "status", ...facts }, true);
  return html`<div class="status-line">
    ${mode ? html`<${ModeButton} mode=${mode} send=${send} />` : null}
    ${activity ? html`<span class="activity"><${Spinner} /> ${activity}</span>` : null}
    <button type="button" class="segments" aria-label="Session details" onClick=${details}
      dangerouslySetInnerHTML=${{ __html: rendered ?? "" }}></button>
  </div>`;
}

/**
 * The mode behind the mark the TUI's composer title gives it, drawn by the host; the bare name
 * until it is. A tap switches to the next mode.
 * @param {{ mode: string, send: (frame: Frame) => boolean }} props
 */
function ModeButton({ mode, send }) {
  const rendered = useRendered({ kind: "mode_name", mode }, true);
  const label = rendered ? { dangerouslySetInnerHTML: { __html: rendered } } : { children: mode };
  return html`<button type="button" class=${`mode mode-${mode}`} aria-label=${`Mode: ${mode}, tap to switch`}
    onClick=${() => send(nextModeFrame(mode))} ...${label} />`;
}
