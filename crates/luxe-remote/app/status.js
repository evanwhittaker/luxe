// @ts-check
// The status line under the composer: the bar's facts (on the left the pull request, the plan and
// the change count; on the right context, cost, the turn's tokens, thinking, model) as the host
// words them and cuts them into slabs, so they read as the TUI's do. The host fits them to the cells
// the line has, dropping what the bar drops first, so a phone keeps one row; a tap opens the details
// sheet with the rest.

import { batch, signal } from "@preact/signals";
import { useLayoutEffect, useRef, useState } from "preact/hooks";
import { MODES } from "./details.js";
import { html } from "./html.js";
import { useRendered } from "./render.js";

/** @import { Signal } from "@preact/signals" */
/** @import { Frame } from "./connection.js" */
/** @typedef {{ facts: Signal<Record<string, any>>, mode: Signal<string>, started: Signal<number> }} State */

/**
 * Nothing reported yet, and the function that folds frames into what is. `started` is when the
 * latest turn began here, by this page's clock: frames carry no time, so a turn caught partway
 * through counts from when the page saw it start.
 */
export function createStatus() {
  /** @type {State} */
  const state = { facts: signal({}), mode: signal(""), started: signal(0) };
  /** @param {Frame} frame */
  const fold = (frame) => batch(() => apply(state, frame));
  return { facts: state.facts, mode: state.mode, started: state.started, apply: fold };
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
    case "turn_started":
      state.started.value = Date.now();
      return update(state, { turn: undefined });
    case "turn_usage": return update(state, { turn: addedUsage(state.facts.value.turn, content) });
  }
}

/**
 * The turn's tokens with one more call's added: the bar shows the turn's, and each frame is one
 * provider call, so a turn that runs twenty tools sends twenty.
 * @param {{ prompt: number, completion: number, cache_read: number } | undefined} turn
 * @param {Record<string, any>} call
 */
export function addedUsage(turn = { prompt: 0, completion: 0, cache_read: 0 }, call) {
  return {
    prompt: turn.prompt + (call.prompt_tokens ?? 0),
    completion: turn.completion + (call.completion_tokens ?? 0),
    cache_read: turn.cache_read + (call.cache_read_tokens ?? 0),
  };
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
 * The session's facts, fitted to the line. The mode leads the row over the composer, and the
 * project and branch head the screen, so the line says none of them.
 * @param {{ facts: Record<string, any>, details: () => void }} props
 */
export function StatusLine({ facts, details }) {
  const line = useRef(/** @type {HTMLDivElement | null} */ (null));
  const cells = useCells(line);
  const rendered = useRendered({ kind: "status", ...facts, ...(cells ? { cells } : {}) }, true);
  return html`<div class="status-line" ref=${line}>
    <span class="cell-probe" aria-hidden="true">0000000000</span>
    <button type="button" class="segments" aria-label="Session details" onClick=${details}
      dangerouslySetInnerHTML=${{ __html: rendered ?? "" }}></button>
  </div>`;
}

/**
 * The cells across the line, as the bar counts its width: the line's width over one cell's,
 * measured from ten of them so a fractional advance doesn't round away.
 * @param {{ current: HTMLElement | null }} line
 */
function useCells(line) {
  const [cells, setCells] = useState(0);
  useLayoutEffect(() => {
    const element = line.current;
    const probe = element?.querySelector(".cell-probe");
    if (!element || !probe || typeof ResizeObserver === "undefined") return;
    const measure = () => setCells(cellsAcross(element.clientWidth, probe.getBoundingClientRect().width / 10));
    const watcher = new ResizeObserver(measure);
    watcher.observe(element);
    measure();
    return () => watcher.disconnect();
  }, []);
  return cells;
}

/** Whole cells in `width`; none while there's nothing to measure. @param {number} width @param {number} cell */
export function cellsAcross(width, cell) {
  return cell > 0 ? Math.floor(width / cell) : 0;
}

/**
 * The mode behind the mark the TUI's composer title gives it, drawn by the host; the bare name
 * until it is. A tap switches to the next mode.
 * @param {{ mode: string, send: (frame: Frame) => boolean }} props
 */
export function ModeButton({ mode, send }) {
  const rendered = useRendered({ kind: "mode_name", mode }, true);
  const label = rendered ? { dangerouslySetInnerHTML: { __html: rendered } } : { children: mode };
  return html`<button type="button" class=${`mode mode-${mode}`} aria-label=${`Mode: ${mode}, tap to switch`}
    onClick=${() => send(nextModeFrame(mode))} ...${label} />`;
}
