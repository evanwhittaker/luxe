// @ts-check
// One session: its transcript, live over the socket, and how the connection stands.

import { signal } from "@preact/signals";
import { useEffect, useMemo, useRef } from "preact/hooks";
import { commandAction, runCommand } from "./commands.js";
import { connect, statusWord } from "./connection.js";
import { html } from "./html.js";
import { Icon } from "./icons.js";
import { Composer } from "./composer.js";
import { DetailsSheet } from "./details.js";
import { DiffSheet, DockToggle } from "./dock.js";
import { createFiles } from "./files.js";
import { createGithub } from "./github.js";
import { createReview } from "./review.js";
import { Transcript } from "./items.js";
import { WIDE, dockVisible, phoneDock, theDock, useWide } from "./layout.js";
import { createListings } from "./mentions.js";
import { createModels } from "./models.js";
import { createPending } from "./pending.js";
import { PairWithLink } from "./pairing.js";
import { PanelButton, ProjectName, ProjectPanel, createPanel } from "./panel.js";
import { PromptCard } from "./prompt.js";
import { folderName } from "./sessions.js";
import { StatusLine, activityLabel, createStatus } from "./status.js";
import { createTranscript } from "./transcript.js";
import { createWorkspace, diffRequest } from "./workspace.js";

const PROTOCOL_VERSION = 2;

/** @import { Frame, Status } from "./connection.js" */
/** @import { Signal } from "@preact/signals" */
/** @typedef {ReturnType<typeof createView>} View */

/** @param {{ sessionId: string }} props */
export function SessionView({ sessionId }) {
  const view = useMemo(createView, [sessionId]);
  const transcriptRef = useRef(/** @type {HTMLElement | null} */ (null));
  const following = useMemo(() => signal(true), [sessionId]);
  useLink(sessionId, view);
  usePinnedScroll(transcriptRef, following);
  return html`
    <${Header} view=${view} sessionId=${sessionId} />
    <main ref=${transcriptRef}>
      <${Transcript} items=${view.transcript.items.value} following=${following.value} waiting=${view.pending.prompts.value.length > 0} />
      <${Ending} status=${view.status.value} problem=${view.problem.value} />
    </main>
    <${Footer} view=${view} />
    <${WorkspaceViews} view=${view} />
  `;
}

/** @param {{ view: View, sessionId: string }} props */
function Header({ view, sessionId }) {
  const dock = theDock();
  const wide = useWide();
  return html`<header class="bar">
    <a class="back" href="#/" aria-label="All sessions"><${Icon} name="back" /></a>
    <span class="title">${view.title.value ? html`<${ProjectName} project=${projectOf(view)} />` : sessionId.slice(0, 8)}</span>
    ${wide ? html`<${DockToggle} shown=${dockVisible(dock.mode.value, wide)} toggle=${() => dock.apply("toggle")} />`
      : html`<${PanelButton} open=${() => view.panel.open()} />`}
    <${Connection} status=${view.status.value} />
  </header>`;
}

/** The sidebar, the diff a tap in its dock opens, and the status line's details. @param {{ view: View }} props */
function WorkspaceViews({ view }) {
  const { workspace } = view;
  const open = openDiff(view);
  return html`
    <${DiffSheet} open=${workspace.diff.value} close=${workspace.close} retry=${() => workspace.diff.value && open(workspace.diff.value.path)}
      ask=${(/** @type {string} */ words) => mentionIn(view, words)} />
    <${ProjectPanel} panel=${view.panel} hooks=${panelHooks(view)} />
    ${view.detailsShown.value ? html`<${DetailsSheet} facts=${view.statusLine.facts.value} mode=${view.statusLine.mode.value}
      listing=${view.models.listing.value} send=${(/** @type {Frame} */ frame) => view.send(frame)} close=${() => (view.detailsShown.value = false)} />` : null}
  `;
}

/** Asks for a changed file's diff, which opens over the page. @param {View} view */
function openDiff(view) {
  return (/** @type {string} */ path) => {
    const request = diffRequest(path);
    view.workspace.opened(request);
    if (!view.send(request)) view.workspace.lost();
  };
}

/**
 * `/dock`'s verb: on a phone, where the sidebar is a sheet, it opens or closes it on the dock;
 * the side and mode, and every verb on a wide screen, go to the rail.
 * @param {View} view @param {string} verb
 */
function workDock(view, verb) {
  const onPhone = matchMedia(WIDE).matches ? null : phoneDock(verb, view.panel.shown.value);
  if (onPhone === "open") return view.panel.open("dock");
  if (onPhone === "close") return view.panel.close();
  theDock().apply(verb);
}

/**
 * Puts `words` in the draft and clears every sheet over the composer, where the question about
 * them is typed next. @param {View} view @param {string} words
 */
function mentionIn(view, words) {
  view.mentioned.value = words;
  view.files.close();
  view.github.close();
  view.review.close();
  view.panel.close();
  view.workspace.close();
}

/** What the sidebar works the session with. @param {View} view @returns {import("./panel.js").PanelHooks} */
function panelHooks(view) {
  /** @param {string} words */
  const mention = (words) => mentionIn(view, words);
  const { listings, files, github, review, workspace } = view;
  const project = projectOf(view);
  const dock = { changes: workspace.changes.value, todos: workspace.todos.value, openDiff: openDiff(view), diffOpen: Boolean(workspace.diff.value) };
  const live = view.status.value === "live";
  return { send: (frame) => view.send(frame), listings, files, github, review, mention, ...dock, project, live };
}

/** The session's folder, its branch and whether the tree has changes. @param {View} view @returns {import("./panel.js").Project} */
function projectOf(view) {
  const identity = view.workspace.project.value;
  return { name: view.title.value, branch: identity?.git_branch ?? null, dirty: Boolean(identity?.git_dirty) };
}

/** Opens the status line's details, asking for the models afresh. @param {View} view */
function openDetails(view) {
  view.models.ask(view.send);
  view.detailsShown.value = true;
}

/**
 * The socket for as long as the view shows this session; `view.send` writes to it.
 * @param {string} sessionId @param {View} view
 */
function useLink(sessionId, view) {
  useEffect(() => {
    const link = connect(sessionId, { onFrame: view.apply, onStatus: (status) => (view.status.value = status) });
    view.send = link.send;
    return () => link.close();
  }, [sessionId]);
}

/** What the sidebar's project views keep: the files, GitHub and the review, each with its open pane. */
function panes() {
  return { files: createFiles(), github: createGithub(), review: createReview() };
}

function createView() {
  const view = {
    transcript: createTranscript(),
    pending: createPending(),
    statusLine: createStatus(),
    models: createModels(),
    listings: createListings(),
    workspace: createWorkspace(),
    ...panes(),
    panel: createPanel(),
    mentioned: signal(/** @type {string | null} */ (null)),
    detailsShown: signal(false),
    title: signal(""),
    status: signal(/** @type {Status} */ ("connecting")),
    problem: signal(""),
    /** @type {(frame: Frame) => boolean} */ send: () => false,
  };
  return Object.assign(view, { apply: (/** @type {Frame} */ frame) => applyFrame(view, frame) });
}

/** What you answer and write with, kept at the foot of the screen. @param {{ view: View }} props */
function Footer({ view }) {
  const { pending } = view;
  /** @param {Frame} frame */
  const send = (frame) => view.send(frame);
  const hooks = { send, details: () => openDetails(view), dock: (/** @type {string} */ verb) => workDock(view, verb) };
  /** @param {string} text */
  const run = (text) => runCommand(commandAction(text, view.statusLine.mode.value), hooks);
  if (view.status.value === "ended" || view.status.value === "unpaired") return null;
  const { busy, items, compacting } = view.transcript;
  const activity = activityLabel(busy.value, items.value, compacting.value);
  return html`<footer class="controls">
    <${PromptCard} prompts=${pending.prompts.value} send=${send} />
    <${Composer} busy=${view.transcript.busy.value} live=${view.status.value === "live"} steering=${pending.steering.value}
      listings=${view.listings} send=${send} run=${run} onSteered=${pending.steered} onDropped=${pending.dropSteering}
      mention=${view.mentioned.value} taken=${() => (view.mentioned.value = null)} />
    <${StatusLine} facts=${view.statusLine.facts.value} mode=${view.statusLine.mode.value} send=${send}
      activity=${activity} details=${() => openDetails(view)} />
  </footer>`;
}

/**
 * @param {Omit<View, "apply">} view The view being made, which `apply` is added to
 * @param {Frame} frame
 */
function applyFrame(view, frame) {
  view.pending.apply(frame);
  view.statusLine.apply(frame);
  view.models.apply(frame);
  view.listings.apply(frame);
  view.workspace.apply(frame);
  view.files.apply(frame);
  view.github.apply(frame);
  view.review.apply(frame);
  if (frame.kind !== "ready") return view.transcript.apply(frame);
  view.title.value = folderName(frame.content.cwd);
  if (frame.content.protocol_version !== PROTOCOL_VERSION) {
    view.problem.value = `Protocol mismatch: luxe uses ${frame.content.protocol_version}, this app ${PROTOCOL_VERSION}.`;
  }
}

/** @param {{ status: Status }} props */
function Connection({ status }) {
  const word = statusWord(status);
  return word ? html`<span class=${`connection ${status}`}>${word}</span>` : null;
}

/** @param {{ status: string, problem: string }} props */
function Ending({ status, problem }) {
  if (problem) return html`<p class="problem">${problem}</p>`;
  if (status === "ended") return html`<p class="notice">Session ended.</p>`;
  if (status !== "unpaired") return null;
  return html`<p class="problem">Device unpaired.</p><${PairWithLink} />`;
}

/**
 * Follow new output while the reader is at the bottom; leave them be once they scroll up. Only
 * scrolling up lets go: items drawn late grow the page under a scroll to the bottom, which would
 * otherwise read as the reader having left it. `following` says which, for the transcript's window.
 * @param {{ current: HTMLElement | null }} ref @param {Signal<boolean>} following
 */
function usePinnedScroll(ref, following) {
  useEffect(() => (ref.current ? followGrowth(ref.current, following) : undefined), [following]);
}

/**
 * Keeps `main`, the page's one scroller, at its bottom as the transcript in it grows or the
 * composer under it does, until stopped.
 * @param {HTMLElement} main @param {Signal<boolean>} following
 */
function followGrowth(main, following) {
  following.value = true;
  let lastY = main.scrollTop;
  const onScroll = () => {
    if (atBottom(main)) following.value = true;
    else if (main.scrollTop < lastY) following.value = false;
    lastY = main.scrollTop;
  };
  const follow = new ResizeObserver(() => following.value && (main.scrollTop = main.scrollHeight));
  main.addEventListener("scroll", onScroll, { passive: true });
  follow.observe(main);
  if (main.firstElementChild) follow.observe(main.firstElementChild);
  return () => {
    main.removeEventListener("scroll", onScroll);
    follow.disconnect();
  };
}

/**
 * Whether `scroller` shows its end, give or take a finger's slop.
 * @param {{ scrollTop: number, clientHeight: number, scrollHeight: number }} scroller
 */
function atBottom({ scrollTop, clientHeight, scrollHeight }) {
  return scrollTop + clientHeight >= scrollHeight - 48;
}
