// @ts-check
// The GitHub view, as the TUI's `/github`: a frame titled with the repository, the section tabs
// across the top, the shown tab's search and rows on the left and the open row's detail on the
// right, its header over an Overview and a Conversation. On a screen too narrow for both, the
// detail takes the list's place until it's put back. The host draws every tab, row, check and
// card; the app only says which, and owns the taps and keys.

import { useEffect, useLayoutEffect, useRef } from "preact/hooks";
import { githubMention, itemSubject, placeOf, reviewable, rowSubject, shownSection, steppedItem, steppedSection, tabsOf } from "./github.js";
import { useRendered } from "./render.js";
import { html, landedIn } from "./html.js";
import { useEscape } from "./dock.js";
import { ChromeMark } from "./marks.js";
import { CursorPiece, ViewFrame, useSplit, useViewKeys } from "./views.js";

/** @import { OpenItem, Reading, Subject } from "./github.js" */
/** @import { Hint } from "./views.js" */
/**
 * @typedef {{ github: ReturnType<typeof import("./github.js").createGithub>, send: (frame: import("./connection.js").Frame) => boolean,
 *   mention: (words: string) => void, review: (subject: Subject) => void, close: () => void }} GithubHooks
 */

/** @param {{ hooks: GithubHooks }} props */
export function GithubView({ hooks }) {
  const { github, send } = hooks;
  const split = useSplit();
  useEffect(() => void github.list(send), []);
  const listing = github.listing.value;
  const shown = listing.sections.length ? shownSection(listing.sections, github.section.value) : null;
  useFirstRow(hooks, shown, split);
  const item = github.open.value;
  const reading = Boolean(item) && !split;
  const hints = githubHints(hooks, shown, item, reading);
  useEscape(reading ? github.close : hooks.close);
  useViewKeys(hints, cursorKeys(hooks, shown));
  const title = listing.repository ? `GitHub · ${listing.repository}` : "GitHub";
  return html`<${ViewFrame} name="github" label="GitHub" title=${title} hints=${hints} place=${placeOf(shown, item?.subject ?? null)}>
    ${shown ? html`<${Tabs} sections=${listing.sections} shown=${shown.name} pick=${(/** @type {string} */ name) => pickSection(hooks, name, split)} />` : null}
    <${Panes} hooks=${hooks} shown=${shown} item=${item} reading=${reading} />
  </${ViewFrame}>`;
}

/** The list and the detail, side by side or, on a phone, the detail in the list's place. @param {{ hooks: GithubHooks, shown: any, item: OpenItem | null, reading: boolean }} props */
function Panes({ hooks, shown, item, reading }) {
  return html`<div class=${reading ? "gh-split reading" : "gh-split"}>
    <div class="gh-list-pane"><${ListBody} hooks=${hooks} shown=${shown} /></div>
    <div class="gh-detail-pane gh-view" onClick=${(/** @type {MouseEvent} */ event) => tapDetail(event, hooks)}>
      ${item ? html`<${Detail} item=${item} hooks=${hooks} />` : html`<p class="diff-note">Nothing open</p>`}</div>
  </div>`;
}

/**
 * On a split screen the first row opens as a list arrives or a tab changes, as the TUI's cursor
 * starts on it with the detail beside; a row opened from elsewhere (a linked pull request) stays.
 * @param {GithubHooks} hooks @param {any} shown @param {boolean} split
 */
function useFirstRow({ github, send }, shown, split) {
  const at = github.listing.value.at;
  useEffect(() => {
    if (!split || !shown?.items?.length) return;
    const open = github.open.value?.subject;
    if (open && shown.items.some((/** @type {any} */ item) => item.url === open.url)) return;
    github.read(itemSubject(shown.items[0], github.listing.value.repository), send);
  }, [shown?.name, at, split]);
}

/** Shows section `name`; on a phone the open row goes with the tab it was on. @param {GithubHooks} hooks @param {string} name @param {boolean} split */
function pickSection({ github }, name, split) {
  github.pick(name);
  if (!split) github.close();
}

/**
 * The hint line, as the TUI's: what the open row can do, the way back to the list on a phone,
 * and close. @param {GithubHooks} hooks @param {any} shown @param {OpenItem | null} item @param {boolean} reading
 * @returns {Hint[]}
 */
function githubHints(hooks, shown, item, reading) {
  const { github } = hooks;
  const here = github.listing.value.repository;
  /** @type {Hint[]} */
  const hints = reading ? [{ key: "←", word: "list", run: github.close }] : [];
  if (item && reviewable(item.subject)) hints.push({ key: "enter", word: "review", run: () => hooks.review(item.subject), tone: "accent" });
  if (item) hints.push({ key: "c", word: "mention", run: () => hooks.mention(githubMention(item.subject, here)) },
    { key: "o", word: "open", run: () => void open(item.subject.url, "_blank", "noopener") });
  if (shown && !reading) hints.push({ key: "tab", word: "section", run: () => github.pick(steppedSection(github.listing.value.sections, shown.name, 1).name) });
  hints.push({ key: "esc", word: "close", run: hooks.close });
  return hints;
}

/** The keys no hint shows: the arrows (and j and k) step the cursor, shift+tab the tab back. @param {GithubHooks} hooks @param {any} shown */
function cursorKeys({ github, send }, shown) {
  const here = github.listing.value.repository;
  const step = (/** @type {number} */ by) => () => {
    const next = steppedItem(shown, github.open.value?.subject ?? null, by);
    if (next) github.read(itemSubject(next, here), send);
  };
  const back = () => shown && github.pick(steppedSection(github.listing.value.sections, shown.name, -1).name);
  return { "↓": step(1), j: step(1), "↑": step(-1), k: step(-1), "shift+tab": back };
}

/**
 * The tabs, drawn by the host with the TUI's mark for each search; plain words until it answers.
 * @param {{ sections: any[], shown: string, pick: (name: string) => void }} props
 */
function Tabs({ sections, shown, pick }) {
  const rendered = useRendered({ kind: "github_tabs", tabs: tabsOf(sections), shown }, false);
  const ref = useShownTab(rendered, shown);
  /** @param {MouseEvent} event */
  const tap = (event) => {
    const tab = /** @type {HTMLElement | null} */ (landedIn(event, "button.gh-tab"));
    if (tab?.dataset.section) pick(tab.dataset.section);
  };
  if (rendered) return html`<div ref=${ref} class="gh-tab-line" onClick=${tap} dangerouslySetInnerHTML=${{ __html: rendered }}></div>`;
  return html`<div ref=${ref} class="gh-tab-line" onClick=${tap}><nav class="gh-tabs" aria-label="GitHub sections">
    ${sections.map(({ name }) => html`<button type="button" key=${name} class="gh-tab" data-section=${name} aria-pressed=${name === shown}>${name}</button>`)}
  </nav></div>`;
}

/** Keeps the shown tab in sight on a tab line too narrow for them all. @param {string | null | undefined} rendered @param {string} shown */
function useShownTab(rendered, shown) {
  const ref = useRef(/** @type {HTMLDivElement | null} */ (null));
  useLayoutEffect(() => {
    ref.current?.querySelector('[aria-pressed="true"]')?.scrollIntoView?.({ block: "nearest", inline: "nearest" });
  }, [rendered, shown]);
  return ref;
}

/** @param {{ hooks: GithubHooks, shown: any }} props */
function ListBody({ hooks, shown }) {
  const { github, send } = hooks;
  const listing = github.listing.value;
  if (listing.state === "lost" && !listing.sections.length) return html`<${Lost} retry=${() => github.list(send)} />`;
  if (!shown) return html`<p class="diff-note">${listing.state === "listed" ? "No sections" : "Loading…"}</p>`;
  return html`<${List} hooks=${hooks} section=${shown} />`;
}

/** @param {{ retry: () => void }} props */
function Lost({ retry }) {
  return html`<p class="diff-note">Connection lost. <button type="button" class="retry" onClick=${retry}>Try again</button></p>`;
}

/** The shown tab's search and rows, the open row on the cursor's slab. @param {{ hooks: GithubHooks, section: any }} props */
function List({ hooks, section }) {
  const { repository, at } = hooks.github.listing.value;
  const rendered = useRendered({ kind: "github_list", repository, section, now: at }, false);
  return html`<${CursorPiece} markup=${rendered ?? ""} attribute="data-url" current=${hooks.github.open.value?.subject.url ?? null}
    className="gh-view" onClick=${(/** @type {MouseEvent} */ event) => tapGithubRow(event, hooks)} />`;
}

/** A tap on a row opens it beside the list, or in its place on a phone. @param {MouseEvent} event @param {GithubHooks} hooks */
export function tapGithubRow(event, { github, send }) {
  const row = landedIn(event, "button.gh-row");
  if (row) github.read(rowSubject(row, github.listing.value.repository), send);
}

/** @param {{ item: OpenItem, hooks: GithubHooks }} props */
function Detail({ item, hooks }) {
  const retry = () => hooks.github.read(item.subject, hooks.send);
  const { overview } = item;
  if (overview.state !== "read") return html`<${Waiting} reading=${overview} retry=${retry} what="overview" />`;
  const shown = item.tab === "overview" ? overview : item.conversation;
  return html`<${Piece} piece=${{ kind: "github_header", overview: overview.value, now: overview.at }} />
    <${DetailTabs} tab=${item.tab} show=${hooks.github.show} said=${overview.value.overview?.conversation_count ?? 0} />
    ${shown.state === "read" ? html`<${Piece} piece=${detailPiece(item)} />` : html`<${Waiting} reading=${shown} retry=${retry} what="conversation" />`}`;
}

/** The shown tab's piece. @param {OpenItem} item */
function detailPiece({ tab, overview, conversation }) {
  if (tab === "overview") return { kind: "github_overview", overview: overview.value, now: overview.at };
  const opened_by_you = overview.value.overview?.viewer?.author === true;
  return { kind: "github_conversation", entries: conversation.value, now: conversation.at, opened_by_you };
}

/** @param {{ reading: Reading, retry: () => void, what: string }} props */
function Waiting({ reading, retry, what }) {
  if (reading.state === "lost") return html`<${Lost} retry=${retry} />`;
  if (reading.state === "failed") return html`<p class="diff-note">${reading.error}</p>`;
  return html`<p class="diff-note">Reading the ${what}…</p>`;
}

/**
 * The detail's tabs, each behind the TUI's mark, Conversation counting what's been said.
 * @param {{ tab: import("./github.js").DetailTab, show: (tab: import("./github.js").DetailTab) => void, said: number }} props
 */
function DetailTabs({ tab, show, said }) {
  /** @type {[import("./github.js").DetailTab, string][]} */
  const tabs = [["overview", "Overview"], ["conversation", said ? `Conversation ${said}` : "Conversation"]];
  return html`<nav class="gh-tabs gh-detail-tabs" role="tablist" aria-label="Views">
    ${tabs.map(([id, title]) => html`<button type="button" role="tab" key=${id} class="gh-tab" aria-selected=${id === tab}
      aria-pressed=${id === tab} onClick=${() => show(id)}><${ChromeMark} mark=${id} className="gh-tab-mark" /><span class="gh-tab-name">${title}</span></button>`)}
  </nav>`;
}

/** @param {{ piece: import("./render.js").Piece }} props */
function Piece({ piece }) {
  const rendered = useRendered(piece, false);
  return html`<div dangerouslySetInnerHTML=${{ __html: rendered ?? "" }}></div>`;
}

/**
 * Taps inside the detail: "open #N" opens that pull request, "more lines in Conversation" shows
 * it, and "Show all" unfolds a long comment. @param {MouseEvent} event @param {GithubHooks} hooks
 */
export function tapDetail(event, { github, send }) {
  const item = github.open.value;
  const linked = landedIn(event, "button.gh-open");
  if (item && linked) return github.read(linkedSubject(item.subject, Number(linked.dataset.number), github.listing.value.repository), send);
  if (landedIn(event, "button.gh-more")) return github.show("conversation");
  landedIn(event, "button.gh-unfold")?.closest(".gh-card")?.classList.remove("folded");
}

/**
 * The pull request `number` in the same repository as `subject`.
 * @param {Subject} subject @param {number} number @param {string | null} here @returns {Subject}
 */
export function linkedSubject(subject, number, here) {
  const repository = subject.repository ?? here;
  return { number, pull_request: true, repository: subject.repository, url: `https://github.com/${repository}/pull/${number}` };
}
