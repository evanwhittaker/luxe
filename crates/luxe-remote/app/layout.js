// @ts-check
// Whether the sidebar shows, and on which side: started from your config's dock, which the bridge
// writes on the page, as the TUI starts from it. `/dock` and the header's button change them while
// the app is open, as `/dock` does in the TUI. The sidebar is the dock's plan and changes with the
// project's views under them: a rail beside the conversation on a wide screen, a sheet over it on a
// phone, behind the header's one button either way.

import { signal } from "@preact/signals";
import { useEffect, useState } from "preact/hooks";

/**
 * @typedef {"auto" | "shown" | "hidden"} DockMode
 * @typedef {"dock" | "files" | "github" | "review"} Part
 */

/**
 * The sidebar's parts, top to bottom, and whether each starts unfolded. GitHub and Review start
 * folded: unfolding one asks the host for it, and GitHub's listing goes out to GitHub, which a
 * rail showing on every iPad session shouldn't do unasked.
 */
export const SIDEBAR_PARTS = /** @type {{ id: Part, title: string, unfolded: boolean }[]} */ ([
  { id: "dock", title: "Dock", unfolded: true },
  { id: "files", title: "Project", unfolded: true },
  { id: "github", title: "GitHub", unfolded: false },
  { id: "review", title: "Review", unfolded: false },
]);

/**
 * What a `/dock` verb does on a phone, where the sidebar is a sheet: open it, close it, or null
 * for the verbs that set the rail's own mode or side.
 * @param {string} verb @param {boolean} open Whether the sheet is open
 * @returns {"open" | "close" | null}
 */
export function phoneDock(verb, open) {
  if (verb === "hide" || (verb === "toggle" && open)) return "close";
  return verb === "show" || verb === "toggle" ? "open" : null;
}

/** Wide and tall enough for the dock beside the conversation; dock.css's breakpoint. */
export const WIDE = "(min-width: 50rem) and (min-height: 30rem)";

/** The verbs `/dock` takes, as the TUI's does; a bare `/dock` toggles. */
export const DOCK_VERBS = ["show", "hide", "auto", "left", "right"];

/** The mode the page was served with, auto when it says nothing known. @param {DOMStringMap} written @returns {DockMode} */
export function startingMode(written) {
  return written.dockMode === "shown" || written.dockMode === "hidden" ? written.dockMode : "auto";
}

/** Whether the dock shows: always when shown, and on a wide screen when auto. @param {DockMode} mode @param {boolean} wide */
export function dockVisible(mode, wide) {
  return mode === "shown" || (mode === "auto" && wide);
}

/**
 * The mode after `verb`: "toggle" flips what's on screen, the rest are `/dock`'s. Null for a verb
 * that doesn't change the mode.
 * @param {DockMode} mode @param {string} verb @param {boolean} wide @returns {DockMode | null}
 */
export function dockAfter(mode, verb, wide) {
  if (verb === "toggle") return dockVisible(mode, wide) ? "hidden" : "shown";
  if (verb === "show") return "shown";
  if (verb === "hide") return "hidden";
  return verb === "auto" ? "auto" : null;
}

/**
 * The dock's mode for the app, and `apply` for a verb. The side lives on the page's own
 * `data-dock-side`, which the stylesheet reads.
 * @param {HTMLElement} page @param {() => boolean} wide
 */
export function createDock(page, wide) {
  const mode = signal(startingMode(page.dataset));
  /** @param {string} verb */
  const apply = (verb) => {
    if (verb === "left" || verb === "right") return void (page.dataset.dockSide = verb);
    const next = dockAfter(mode.value, verb, wide());
    if (next) mode.value = next;
  };
  return { mode, apply };
}

/** @type {ReturnType<typeof createDock> | undefined} */
let appDock;

/** The one dock the app's sessions share, made on first use. */
export function theDock() {
  appDock ??= createDock(document.documentElement, () => matchMedia(WIDE).matches);
  return appDock;
}

/** Whether the screen is wide, following a rotation or a resized window. */
export function useWide() {
  const [wide, setWide] = useState(() => matchMedia(WIDE).matches);
  useEffect(() => {
    const query = matchMedia(WIDE);
    const change = () => setWide(query.matches);
    query.addEventListener("change", change);
    return () => query.removeEventListener("change", change);
  }, []);
  return wide;
}
