// @ts-check
// The sidebar: the dock's plan and changes, then the project's views (its file tree, GitHub,
// Review), in one column that scrolls, with a row of jumps at its top. On a wide screen it's a rail
// beside the conversation, as the TUI keeps FILES in its dock, shown and hidden with the dock; a
// file, a pull request or a review chapter opens in a pane beside it. On a phone it's a sheet over
// the page, behind the header's one button, and a pane covers it. One pane at a time.

import { signal } from "@preact/signals";
import { useLayoutEffect, useRef } from "preact/hooks";
import { DockPart, useEscape } from "./dock.js";
import { FilePeek, FilesTab } from "./files-view.js";
import { GithubPeek, GithubTab } from "./github-view.js";
import { html } from "./html.js";
import { Icon } from "./icons.js";
import { SIDEBAR_PARTS, dockVisible, theDock, useWide } from "./layout.js";
import { useRendered } from "./render.js";
import { ReviewPeek, ReviewTab } from "./review-view.js";

/** @import { Frame } from "./connection.js" */
/** @import { Listings } from "./picker.js" */
/**
 * @typedef {import("./layout.js").Part} Part
 * @typedef {ReturnType<typeof createPanel>} Panel
 * @typedef {{ send: (frame: Frame) => boolean, listings: Listings, files: ReturnType<typeof import("./files.js").createFiles>,
 *   github: ReturnType<typeof import("./github.js").createGithub>, review: ReturnType<typeof import("./review.js").createReview>,
 *   mention: (words: string) => void, changes: any, todos: any, openDiff: (path: string) => void, diffOpen: boolean,
 *   project: Project, live: boolean }} PanelHooks
 * @typedef {{ name: string, branch: string | null, dirty: boolean }} Project
 */

/** @type {Record<Part, (props: { hooks: PanelHooks }) => any>} */
const PART_VIEWS = { dock: DockPart, files: FilesTab, github: GithubTab, review: ReviewTab };

/**
 * The sidebar's state: whether a phone's sheet is open, which parts are unfolded, and the part to
 * bring into view next. A jump unfolds what it lands on, so it never lands on a bare heading.
 */
export function createPanel() {
  const shown = signal(false);
  const unfolded = signal(new Set(SIDEBAR_PARTS.filter((part) => part.unfolded).map((part) => part.id)));
  const target = signal(/** @type {Part | null} */ (null));
  /** @param {Part} part */
  const jump = (part) => {
    unfolded.value = new Set([...unfolded.value, part]);
    target.value = part;
  };
  /** @param {Part} part */
  const fold = (part) => (unfolded.value = toggled(unfolded.value, part));
  const open = (/** @type {Part | null} */ part = null) => ((shown.value = true), part && jump(part));
  return { shown, unfolded, target, jump, fold, open, close: () => (shown.value = false) };
}

/** `parts` with `part` folded if it was unfolded, and unfolded if not. @param {Set<Part>} parts @param {Part} part */
function toggled(parts, part) {
  const next = new Set(parts);
  if (!next.delete(part)) next.add(part);
  return next;
}

/** The phone header's button that opens the sidebar. @param {{ open: () => void }} props */
export function PanelButton({ open }) {
  return html`<button type="button" class="panel-button" aria-label="Dock and project" onClick=${open}><${Icon} name="menu" /></button>`;
}

/** @param {{ panel: Panel, hooks: PanelHooks }} props */
export function ProjectPanel({ panel, hooks }) {
  const wide = useWide();
  const shown = wide ? dockVisible(theDock().mode.value, wide) : panel.shown.value;
  // an open file, item or diff takes Escape first; a rail beside the conversation isn't closed by it
  useEscape(shown && !wide && !paneOpen(hooks) ? panel.close : null);
  return html`${shown ? html`<${Sidebar} panel=${panel} hooks=${hooks} wide=${wide} />` : null}
  <${FilePeek} hooks=${hooks} wide=${wide} />
  <${GithubPeek} hooks=${hooks} wide=${wide} />
  <${ReviewPeek} hooks=${hooks} wide=${wide} />`;
}

/** @param {{ panel: Panel, hooks: PanelHooks, wide: boolean }} props */
function Sidebar({ panel, hooks, wide }) {
  const sheet = useRef(/** @type {HTMLElement | null} */ (null));
  useJump(panel, sheet);
  return html`<div class="sheet panel" ref=${sheet} role=${wide ? "complementary" : "dialog"} aria-modal=${!wide}
      aria-label="Dock and project">
    <div class="panel-top">
      ${wide ? null : html`<${SheetBar} project=${hooks.project} close=${panel.close} />`}
      <${Jumps} jump=${panel.jump} />
    </div>
    <div class="sheet-body">${SIDEBAR_PARTS.map(({ id, title }) =>
      html`<${PartView} key=${id} id=${id} title=${title} panel=${panel} hooks=${hooks} />`)}</div>
  </div>`;
}

/** The phone sheet's bar: the project, and the way back to the conversation. @param {{ project: Project, close: () => void }} props */
function SheetBar({ project, close }) {
  return html`<header class="bar">
    <span class="title"><${ProjectName} project=${project} /></span>
    <button type="button" class="close" aria-label="Close the sidebar" onClick=${close}><${Icon} name="close" /></button>
  </header>`;
}

/** The row of jumps, one a part. @param {{ jump: (part: Part) => void }} props */
function Jumps({ jump }) {
  return html`<nav class="jumps" aria-label="Sidebar parts">
    ${SIDEBAR_PARTS.map(({ id, title }) => html`<button type="button" key=${id} onClick=${() => jump(id)}>${title}</button>`)}
  </nav>`;
}

/**
 * One part: the dock under the host's own headings, or a project view under a heading that folds
 * it. A view is only built while it's unfolded and the link is up: each asks the host for what it
 * shows as it's built, and the rail is on screen from before the link is, and after one drops.
 * @param {{ id: Part, title: string, panel: Panel, hooks: PanelHooks }} props
 */
function PartView({ id, title, panel, hooks }) {
  const View = PART_VIEWS[id];
  if (id === "dock") return html`<section class="part" data-part="dock" aria-label=${title}><${View} hooks=${hooks} /></section>`;
  const unfolded = panel.unfolded.value.has(id);
  return html`<section class="part" data-part=${id} aria-label=${title}>
    <h2 class="part-head"><button type="button" aria-expanded=${unfolded} onClick=${() => panel.fold(id)}>
      <span class="badge">${title}</span><span class="part-fold">${unfolded ? "hide" : "show"}</span></button></h2>
    ${unfolded && hooks.live ? html`<${View} hooks=${hooks} />` : null}
  </section>`;
}

/**
 * Brings the part a jump asked for to the top of the sidebar, under the jumps, once it's drawn.
 * @param {Panel} panel @param {{ current: HTMLElement | null }} sheet
 */
function useJump(panel, sheet) {
  const target = panel.target.value;
  useLayoutEffect(() => {
    const part = target && sheet.current?.querySelector(`[data-part="${target}"]`);
    if (!(part instanceof HTMLElement) || !sheet.current) return;
    const top = /** @type {HTMLElement | null} */ (sheet.current.querySelector(".panel-top"));
    sheet.current.scrollTop = part.offsetTop - (top?.offsetHeight ?? 0);
    panel.target.value = null;
  }, [target]);
}

/** Whether a file, an item, a chapter or a diff is open over the sidebar. @param {PanelHooks} hooks */
const paneOpen = ({ files, github, review, diffOpen }) =>
  Boolean(files.open.value || github.open.value || review.open.value || diffOpen);

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
