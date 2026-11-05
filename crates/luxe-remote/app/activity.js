// @ts-check
// The row over the composer, as the TUI's: the mode leads, a tap switches it, and while a turn runs
// the activity bar follows (a comet of the mode's shades sweeping a rail it lights as it passes),
// then how long the turn has run. The Stop button is the interrupt here, so there's no chord to
// name. The sweep is luxe-tui's `activity_bar`, step for step, in the theme's derived shades.

import { useEffect, useState } from "preact/hooks";
import { html } from "./html.js";
import { ModeButton } from "./status.js";

/** @import { Frame } from "./connection.js" */
/** @typedef {{ glyph: string, ramp: "trail" | "rail", step: number }} Cell */

/** Cells across, as `ACTIVITY_BAR_WIDTH`. */
export const BAR_CELLS = 24;
/** The TUI's tick, about 30 a second. */
const TICK_MS = 1000 / 30;
/** Ticks per cell, as a fraction, and ticks the rail stays empty between legs: the TUI's pace. */
const CELL_FRAMES = [2, 1];
const HOLD_FRAMES = 9;
/** Shades in each of the theme's ramps (`ACTIVITY_TRAIL_LEN`). */
const SHADES = 10;
/** `ActivityGlyphs::BARS`: lit units heaviest first, thinning onto the rail. */
const LIT = ["▌", "▍", "▎", "▏"];
const TRACK = "▏";

/**
 * The mode, then the bar and the elapsed time while a turn runs.
 * @param {{ mode: string, busy: boolean, started: number, send: (frame: Frame) => boolean }} props
 */
export function ActivityRow({ mode, busy, started, send }) {
  if (!mode && !busy) return null;
  return html`<div class=${`activity-row mode-${mode || "build"}`}>
    ${mode ? html`<${ModeButton} mode=${mode} send=${send} />` : null}
    ${busy ? html`<${Activity} started=${started} plan=${mode === "plan"} />` : null}
  </div>`;
}

/** @param {{ started: number, plan: boolean }} props */
function Activity({ started, plan }) {
  const now = useTicking();
  const [shown] = useState(() => Date.now());
  // the later of the two: a start left from an earlier turn is older than the bar, and a turn that
  // follows straight on from the last keeps the bar up but restarts the count
  const from = Math.max(started, shown);
  const cells = activityCells(Math.floor((now - from) / TICK_MS), BAR_CELLS, SHADES);
  // one still label, since a live region would read the clock out every second
  return html`<span class="activity" role="img" aria-label="Working">
    <span class="activity-bar" aria-hidden="true">${cells.map((cell) => html`<span style=${shadeStyle(cell, plan)}>${cell.glyph}</span>`)}</span>
    <span class="elapsed">${elapsedCompact(now - from)}</span>
  </span>`;
}

/**
 * A cell's color: the theme's derived shade, plan's own comet while planning, and the accent or
 * the border where a theme came without them.
 * @param {Cell} cell @param {boolean} plan
 */
export function shadeStyle(cell, plan) {
  if (cell.ramp === "rail") return `color: var(--luxe-rail-${cell.step}, var(--luxe-border-muted, #2e2b27))`;
  const ramp = plan ? "trail-plan" : "trail";
  return `color: var(--luxe-${ramp}-${cell.step}, var(--mode, var(--luxe-accent, #ddc084)))`;
}

/** The time, at the TUI's tick, for as long as the caller is shown. */
function useTicking() {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), TICK_MS);
    return () => clearInterval(timer);
  }, []);
  return now;
}

/**
 * `12s`, `3m 04s`, `1h 02m`, as `format_elapsed_compact` words a turn's length.
 * @param {number} ms
 */
export function elapsedCompact(ms) {
  const secs = Math.max(0, Math.floor(ms / 1000));
  const pad = (/** @type {number} */ n) => String(n).padStart(2, "0");
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${pad(secs % 60)}s`;
  return `${Math.floor(secs / 3600)}h ${pad(Math.floor((secs % 3600) / 60))}m`;
}

/**
 * One frame of the bar: each cell's glyph and which shade of which ramp it wears.
 * @param {number} frame @param {number} count @param {number} shades @returns {Cell[]}
 */
export function activityCells(frame, count, shades) {
  const comet = Math.min(Math.max(Math.floor((count * 2) / 5), 2), shades, count);
  if (count === 0 || comet === 0) return [];
  const at = sweep(frame, count, comet);
  const halo = Math.max(Math.floor(count / 2), 2);
  return Array.from({ length: count }, (_, index) => cellAt(at, index, comet, halo, shades));
}

/**
 * @param {{ head: number, forward: boolean } | null} at @param {number} index @param {number} comet
 * @param {number} halo @param {number} shades @returns {Cell}
 */
function cellAt(at, index, comet, halo, shades) {
  if (!at) return { glyph: TRACK, ramp: "rail", step: shades - 1 };
  const behind = at.forward ? at.head - index : index - at.head;
  if (behind >= 0 && behind < comet) {
    return { glyph: LIT[rampStep(comet, behind, LIT.length)], ramp: "trail", step: rampStep(comet, behind, shades) };
  }
  const away = Math.min(Math.abs(index - at.head), halo);
  return { glyph: TRACK, ramp: "rail", step: rampStep(halo + 1, away, shades) };
}

/**
 * Where the comet's head is, or null in the hold between legs. A leg runs the head from one edge
 * to `comet` cells past the other, so the tail clears the rail before the hold.
 * @param {number} frame @param {number} count @param {number} comet
 */
function sweep(frame, count, comet) {
  const steps = count + comet - 1;
  const leg = Math.ceil((steps * CELL_FRAMES[0]) / CELL_FRAMES[1]);
  const phase = frame % (2 * (leg + HOLD_FRAMES));
  const travelled = (/** @type {number} */ p) => Math.min(Math.floor((p * CELL_FRAMES[1]) / CELL_FRAMES[0]), steps - 1);
  if (phase < leg) return { head: travelled(phase), forward: true };
  if (phase < leg + HOLD_FRAMES) return null;
  if (phase < 2 * leg + HOLD_FRAMES) return { head: count - 1 - travelled(phase - leg - HOLD_FRAMES), forward: false };
  return null;
}

/**
 * The shade `offset` cells into a run of `span` takes from a ramp of `len`, as `ramp_step`.
 * @param {number} span @param {number} offset @param {number} len
 */
function rampStep(span, offset, len) {
  const last = len - 1;
  if (span <= 1 || last <= 0) return 0;
  return Math.min(Math.round((offset / (span - 1)) * last), last);
}
