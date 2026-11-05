// @ts-check
// The model and thinking buttons in the composer's foot. Each opens its list over the box, as the
// TUI's pickers open over its composer, so switching doesn't take the trip to the details sheet. A
// pick goes to the session, and the list closes; what's marked current is what the session last
// reported, so a refused switch never looks made.

import { useState } from "preact/hooks";
import { useEscape } from "./dock.js";
import { html } from "./html.js";
import { ChromeMark } from "./marks.js";
import { isCurrent, selectModelFrame, thinkingFrame, thinkingLevels } from "./models.js";

/** @import { Frame } from "./connection.js" */
/** @import { Listing, ModelChoice, CurrentModel } from "./models.js" */
/** @typedef {"model" | "thinking"} Which */
/** @typedef {{ key: string, name: string, note?: string, current: boolean, frame: Frame }} Choice */

/**
 * @param {{ facts: Record<string, any>, listing: Listing, ask: () => void, live: boolean,
 *   send: (frame: Frame) => boolean }} props
 */
export function QuickPicks({ facts, listing, ask, live, send }) {
  const [open, setOpen] = useState(/** @type {Which | null} */ (null));
  /** @param {Which} which */
  const toggle = (which) => {
    // asked each time, as the sheet does: the last answer shows meanwhile
    if (open !== which) ask();
    setOpen(open === which ? null : which);
  };
  const choices = open ? quickChoices(open, listing, facts) : [];
  return html`<${Flyout} which=${open} choices=${choices} state=${listing.state} close=${() => setOpen(null)}
      pick=${(/** @type {Frame} */ frame) => send(frame) && setOpen(null)} />
    <${QuickButton} which="model" label=${`Model: ${facts.model?.display_name ?? facts.model?.model ?? "none"}`}
      open=${open} live=${live} toggle=${toggle} />
    <${QuickButton} which="thinking" label=${`Thinking: ${facts.thinking ?? "unknown"}`}
      open=${open} live=${live} toggle=${toggle} />`;
}

/**
 * @param {{ which: Which, label: string, open: Which | null, live: boolean,
 *   toggle: (which: Which) => void }} props
 */
function QuickButton({ which, label, open, live, toggle }) {
  // the status line's own mark for the fact, so the button and the line beside it match
  return html`<button type="button" class=${`quick quick-${which}`} aria-label=${label} aria-expanded=${open === which}
    disabled=${!live} onClick=${() => toggle(which)}><${ChromeMark} mark=${which} className="quick-mark" drawn=${which} /></button>`;
}


/**
 * What `which` offers, each with the frame that picks it: the listed models, or the levels the
 * running model honors.
 * @param {Which} which @param {Listing} listing @param {Record<string, any>} facts @returns {Choice[]}
 */
export function quickChoices(which, listing, facts) {
  /** @type {CurrentModel | undefined} */
  const current = facts.model;
  if (which === "thinking") {
    return thinkingLevels(listing.models, current).map((level) => ({
      key: level, name: level, current: level === facts.thinking, frame: thinkingFrame(level),
    }));
  }
  return listing.models.map((/** @type {ModelChoice} */ choice) => ({
    key: `${choice.provider}/${choice.model}`, name: choice.name, note: choice.provider,
    current: isCurrent(choice, current), frame: selectModelFrame(choice),
  }));
}

/**
 * @param {{ which: Which | null, choices: Choice[], state: string, close: () => void,
 *   pick: (frame: Frame) => void }} props
 */
function Flyout({ which, choices, state, close, pick }) {
  useEscape(which ? close : null);
  if (!which) return null;
  const label = which === "model" ? "Models" : "Thinking levels";
  return html`<ul class="picker quick-flyout" role="listbox" aria-label=${label}>
    ${choices.length ? null : html`<li class="dim">${emptyNote(state)}</li>`}
    ${choices.map((choice) => html`<li key=${choice.key} role="option" aria-selected=${choice.current}>
      <button type="button" onClick=${() => pick(choice.frame)}>
        <span class=${choice.current ? "name" : ""}>${choice.name}</span>${choice.note ? html` <span class="dim">${choice.note}</span>` : null}
      </button></li>`)}
  </ul>`;
}

/** What an empty list is waiting on. @param {string} state */
export function emptyNote(state) {
  if (state === "unsupported") return "Update luxe on your computer to switch from here.";
  if (state === "lost") return "Not connected.";
  if (state === "asking" || state === "idle") return "Loading…";
  return "Nothing else to pick.";
}
