// @ts-check
// The sidebar: the plan, the project's files (where a changed file opens its diff), then the model
// and the session, in one column that scrolls, as the TUI's dock. On a wide screen it's a rail
// beside the conversation, shown and hidden with the dock, its first heading on the conversation
// bar's line; a file or a diff opens in a pane beside it. On a phone it's a sheet over the page,
// behind the header's button, and a pane covers it. GitHub and Review open over the whole screen
// instead (views.js).

import { signal } from "@preact/signals";
import { ModelPart, PlanPart, SessionPart, useEscape } from "./dock.js";
import { FilePeek, FilesTab } from "./files-view.js";
import { html } from "./html.js";
import { Icon } from "./icons.js";
import { SIDEBAR_PARTS, dockVisible, theDock, useWide } from "./layout.js";
import { useRendered } from "./render.js";

/** @import { Frame } from "./connection.js" */
/** @import { Listings } from "./picker.js" */
/**
 * @typedef {import("./layout.js").Part} Part
 * @typedef {ReturnType<typeof createPanel>} Panel
 * @typedef {{ send: (frame: Frame) => boolean, listings: Listings, files: ReturnType<typeof import("./files.js").createFiles>,
 *   mention: (words: string) => void, changes: any, todos: any, openDiff: (path: string) => void, closeDiff: () => void, diffOpen: boolean,
 *   project: Project, live: boolean, facts: Record<string, any>, mode: string }} PanelHooks
 * @typedef {{ name: string, branch: string | null, dirty: boolean }} Project
 */

/** @type {Record<Part, (props: { hooks: PanelHooks }) => any>} */
const PART_VIEWS = {
  plan: PlanPart, files: FilesTab, model: ModelPart, session: SessionPart,
};

/** The sidebar's state: whether a phone's sheet is open, and which parts are unfolded. */
export function createPanel() {
  const shown = signal(false);
  const unfolded = signal(new Set(SIDEBAR_PARTS.filter((part) => part.unfolded).map((part) => part.id)));
  /** @param {Part} part */
  const fold = (part) => (unfolded.value = toggled(unfolded.value, part));
  return { shown, unfolded, fold, open: () => (shown.value = true), close: () => (shown.value = false) };
}

/** `parts` with `part` folded if it was unfolded, and unfolded if not. @param {Set<Part>} parts @param {Part} part */
function toggled(parts, part) {
  const next = new Set(parts);
  if (!next.delete(part)) next.add(part);
  return next;
}

/** The phone header's button that opens the sidebar. @param {{ open: () => void }} props */
export function PanelButton({ open }) {
  return html`<button type="button" class="panel-button" aria-label="Sidebar" onClick=${open}><${Icon} name="menu" /></button>`;
}

/** @param {{ panel: Panel, hooks: PanelHooks }} props */
export function ProjectPanel({ panel, hooks }) {
  const wide = useWide();
  const shown = wide ? dockVisible(theDock().mode.value, wide) : panel.shown.value;
  // an open file, item or diff takes Escape first; a rail beside the conversation isn't closed by it
  useEscape(shown && !wide && !paneOpen(hooks) ? panel.close : null);
  return html`${shown ? html`<${Sidebar} panel=${panel} hooks=${hooks} wide=${wide} />` : null}
  <${FilePeek} hooks=${hooks} wide=${wide} />`;
}

/** @param {{ panel: Panel, hooks: PanelHooks, wide: boolean }} props */
function Sidebar({ panel, hooks, wide }) {
  return html`<div class="sheet panel" role=${wide ? "complementary" : "dialog"} aria-modal=${!wide} aria-label="Sidebar">
    ${wide ? null : html`<${SheetBar} project=${hooks.project} close=${panel.close} />`}
    <div class="sheet-body">${SIDEBAR_PARTS.map(({ id, title, folds }) =>
      html`<${PartView} key=${id} id=${id} title=${title} folds=${folds} panel=${panel} hooks=${hooks} />`)}</div>
  </div>`;
}

/** The phone sheet's bar: the project, and the way back to the conversation. @param {{ project: Project, close: () => void }} props */
function SheetBar({ project, close }) {
  return html`<header class="bar">
    <span class="title"><${ProjectName} project=${project} /></span>
    <button type="button" class="close" aria-label="Close the sidebar" onClick=${close}><${Icon} name="close" /></button>
  </header>`;
}

/**
 * One part: a dock section the host draws whole, which heads itself and is left out while empty,
 * or a project view under a heading that folds it. A view is only built while it's unfolded and
 * the link is up: each asks the host for what it shows as it's built, and the rail is on screen
 * from before the link is, and after one drops. Each view scrolls in a box of its own, as the
 * TUI's dock gives each section a budget, so a deep tree can't push the parts under it away.
 * @param {{ id: Part, title: string, folds: boolean, panel: Panel, hooks: PanelHooks }} props
 */
function PartView({ id, title, folds, panel, hooks }) {
  const View = PART_VIEWS[id];
  if (!folds) return html`<${View} hooks=${hooks} />`;
  const unfolded = panel.unfolded.value.has(id);
  return html`<section class="part" data-part=${id} aria-label=${title}>
    <h2 class="part-head"><button type="button" aria-expanded=${unfolded} onClick=${() => panel.fold(id)}>
      <span class="badge">${title}</span><span class="part-fold">${unfolded ? "hide" : "show"}</span></button></h2>
    ${unfolded && hooks.live ? html`<div class="part-body"><${View} hooks=${hooks} /></div>` : null}
  </section>`;
}

/** Whether a file or a diff is open over the sidebar. @param {PanelHooks} hooks */
const paneOpen = ({ files, diffOpen }) => Boolean(files.open.value || diffOpen);

/**
 * The folder the session works in and its branch, as the TUI's project section has them, in the
 * session's header and the sidebar's. The host draws it so the glyphs are the TUI's, at the tree's
 * tier; the bare name stands in until it does.
 * @param {{ project: Project }} props
 */
export function ProjectName({ project }) {
  const rendered = useRendered({ kind: "project_name", ...project }, true);
  if (!project.name) return null;
  if (!rendered) return html`<span class="project-name">${project.name}</span>`;
  return html`<span class="project-name" dangerouslySetInnerHTML=${{ __html: rendered }}></span>`;
}
