// @ts-check
// The Review view, as the TUI's Guided Review desk: no frame, the guide pane headed by the REVIEW
// slab and the chapter's place, the open chapter beside it with its notes, diffs and comments. On
// a screen too narrow for both, a tapped chapter takes the guide's place until it's put back, as
// the TUI's narrow desk shows the diff alone. Next opens the chapter after, reviewed marks it
// walked as reading to its end does in the TUI, and comment writes on the picked lines or the
// chapter.

import { useEffect, useState } from "preact/hooks";
import { chapterPiece, guidePiece, nextChapter, steppedChapter, targetName } from "./review.js";
import { useRendered } from "./render.js";
import { html, landedIn } from "./html.js";
import { useEscape } from "./dock.js";
import { PickablePiece } from "./lines-view.js";
import { CommentSheet, Unsent, composeAt, composedEdit, tapCard } from "./comments-view.js";
import { CursorPiece, ViewFrame, useSplit, useViewKeys } from "./views.js";

/** @import { ReviewShown } from "./review.js" */
/** @import { Composing } from "./comments-view.js" */
/** @import { Hint } from "./views.js" */
/**
 * @typedef {{ review: ReturnType<typeof import("./review.js").createReview>, send: (frame: import("./connection.js").Frame) => boolean,
 *   mention: (words: string) => void, close: () => void }} ReviewHooks
 * @typedef {(composing: Composing | null) => void} Compose
 */

/** @param {{ hooks: ReviewHooks }} props */
export function ReviewView({ hooks }) {
  const { review } = hooks;
  const split = useSplit();
  useEffect(() => void (review.shown.value.state !== "loading" && review.ask(hooks.send)), []);
  const shown = review.shown.value;
  const id = review.open.value;
  useFirstChapter(hooks, split);
  const [composing, compose] = useState(/** @type {Composing | null} */ (null));
  useEffect(() => compose(null), [id]);
  const reading = Boolean(id) && !split;
  const hints = shown.review ? reviewHints(hooks, id, reading, compose) : [{ key: "esc", word: "close", run: hooks.close }];
  useEscape(composing ? () => compose(null) : reading ? review.close : hooks.close);
  useViewKeys(composing ? [] : hints, composing ? {} : chapterKeys(hooks, id, split));
  return html`<${ViewFrame} name="review" label="Review" hints=${hints} place=${shown.review && id ? placeOf(shown.review, id) : ""}>
    ${shown.review ? html`<${Desk} hooks=${hooks} id=${id} reading=${reading} composing=${composing} compose=${compose} />`
      : html`<div class="rv-empty"><${Empty} shown=${shown} hooks=${hooks} /></div>`}
  </${ViewFrame}>`;
}

/** On a split screen the first chapter opens with the review, as the desk opens on its first step. @param {ReviewHooks} hooks @param {boolean} split */
function useFirstChapter({ review }, split) {
  const first = review.shown.value.review?.chapters?.[0]?.id ?? null;
  useEffect(() => {
    if (split && first && !review.open.value) review.pick(first);
  }, [first, split]);
}

/** Where chapter `id` is among the review's, as the hint line's `1/4`. @param {any} review @param {string} id */
function placeOf(review, id) {
  const at = review.chapters.findIndex((/** @type {any} */ chapter) => chapter.id === id);
  return at < 0 ? "" : `${at + 1}/${review.chapters.length}`;
}

/**
 * The hint line, as the desk's: the way back to the guide on a phone, next, reviewed, comment,
 * and close. @param {ReviewHooks} hooks @param {string | null} id @param {boolean} reading @param {Compose} compose
 * @returns {Hint[]}
 */
function reviewHints({ review, send, close }, id, reading, compose) {
  const shown = review.shown.value;
  /** @type {Hint[]} */
  const hints = reading ? [{ key: "←", word: "guide", run: review.close }] : [];
  const next = id ? nextChapter(shown.review, id) : null;
  if (next) hints.push({ key: "space", word: "next", run: () => review.pick(next), tone: "accent" });
  if (id) {
    const walked = shown.walked.includes(id);
    hints.push({ key: "r", word: walked ? "reviewed" : "mark reviewed", run: () => review.walk(id, send), tone: walked ? "accent" : undefined },
      { key: "c", word: "comment", run: () => compose(composeAt(shown.review, id, null)) });
  }
  hints.push({ key: "esc", word: "close", run: close });
  return hints;
}

/**
 * The keys no hint shows: the arrows step through the chapters on a split screen.
 * @param {ReviewHooks} hooks @param {string | null} id @param {boolean} split @returns {Record<string, () => void>}
 */
function chapterKeys({ review }, id, split) {
  const step = (/** @type {number} */ by) => () => {
    const to = id ? steppedChapter(review.shown.value.review, id, by) : null;
    if (to) review.pick(to);
  };
  return split ? { "→": step(1), "←": step(-1), j: step(1), k: step(-1) } : { "→": step(1) };
}

/**
 * The guide pane beside the open chapter, or on a phone one in the other's place.
 * @param {{ hooks: ReviewHooks, id: string | null, reading: boolean, composing: Composing | null, compose: Compose }} props
 */
function Desk({ hooks, id, reading, composing, compose }) {
  return html`<div class=${reading ? "rv-desk reading" : "rv-desk"}>
    <aside class="rv-guide-pane" aria-label="Guide"><${GuidePane} hooks=${hooks} id=${id} /></aside>
    <section class="rv-chapter-pane" aria-label="Chapter">
      ${id ? html`<${Chapter} hooks=${hooks} id=${id} composing=${composing} compose=${compose} />` : html`<p class="diff-note">Pick a chapter</p>`}
    </section>
  </div>`;
}

/** The desk's head, what's under review, comments not sent, then the guide. @param {{ hooks: ReviewHooks, id: string | null }} props */
function GuidePane({ hooks, id }) {
  const shown = hooks.review.shown.value;
  const rendered = useRendered(guidePiece(shown), false);
  const place = id ? placeOf(shown.review, id).replace("/", " of ") : "";
  return html`<header class="rv-desk-head"><span class="badge rv-badge">Review</span>
      ${place ? html`<span class="rv-step">Chapter ${place}</span>` : null}</header>
    <p class="rv-target">${targetName(shown.review.target)}</p>
    <${Unsent} shown=${shown} hooks=${hooks} />
    ${shown.problem ? html`<p class="diff-note">${shown.problem}</p>` : null}
    <${CursorPiece} markup=${rendered ?? ""} attribute="data-chapter" current=${id} className="rv-view"
      onClick=${(/** @type {MouseEvent} */ event) => tapChapter(event, hooks)} />`;
}

/** A tap on a chapter opens it beside the guide, or in its place on a phone. @param {MouseEvent} event @param {ReviewHooks} hooks */
export function tapChapter(event, { review }) {
  const chapter = /** @type {HTMLElement | null} */ (landedIn(event, "[data-chapter]"));
  if (chapter?.dataset.chapter) review.pick(chapter.dataset.chapter);
}

/**
 * The open chapter, with the comment composer at its foot while one is written.
 * @param {{ hooks: ReviewHooks, id: string, composing: Composing | null, compose: Compose }} props
 */
function Chapter({ hooks, id, composing, compose }) {
  const { review, send } = hooks;
  const shown = review.shown.value;
  const submit = (/** @type {string} */ body, /** @type {boolean} */ question, /** @type {boolean} */ now) =>
    composing && (review.edit(send, composedEdit(composing, body, question, now)), compose(null));
  return html`<div class="rv-view" key=${id} onClick=${(/** @type {MouseEvent} */ event) => tapCard(event, hooks, compose)}>
      <${PickablePiece} piece=${chapterPiece(shown, id)} ask=${hooks.mention} comment=${(/** @type {any} */ pick) => compose(composeAt(shown.review, id, pick))} /></div>
    ${composing ? html`<${CommentSheet} key=${composing.heading} composing=${composing} submit=${submit} cancel=${() => compose(null)} />` : null}`;
}

/** What the view says with no review to show, and the way to get one. @param {{ shown: ReviewShown, hooks: ReviewHooks }} props */
function Empty({ shown, hooks }) {
  const { review, send } = hooks;
  const again = (/** @type {string} */ words, /** @type {() => void} */ act) =>
    html`<button type="button" class="retry" onClick=${act}>${words}</button>`;
  switch (shown.state) {
    case "lost": return html`<p class="diff-note">Connection lost. ${again("Try again", () => review.ask(send))}</p>`;
    case "failed": return html`<p class="diff-note">${shown.error} ${again("Try again", () => review.load(send))}</p>`;
    case "loading": return html`<p class="diff-note">Reading the changes…</p>`;
    case "shown": return html`<p class="diff-note">No review is open. ${again("Review changes", () => review.load(send, { kind: "working_tree" }))}</p>`;
    default: return html`<p class="diff-note">Loading…</p>`;
  }
}
