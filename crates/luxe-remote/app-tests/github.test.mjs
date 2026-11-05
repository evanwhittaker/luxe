// @ts-check

import assert from "node:assert/strict";
import { test } from "node:test";
import { applyItem, applyListed, createGithub, githubMention, placeOf, reviewable, rowSubject, shownSection, steppedItem, steppedSection, tabsOf } from "../app/github.js";
import { linkedSubject, tapDetail, tapGithubRow } from "../app/github-view.js";

/** @import { OpenItem, GithubListing } from "../app/github.js" */

/** A `github_listed` frame's content, cut from a live session to one section and one row. */
const LISTED = {
  repository: "nora-quinn/luxe-dev",
  sections: [
    { name: "Review requested", items: [], error: null, total: 0 },
    { name: "Pull requests", items: [{ number: 211, title: "chore: refresh the embedded models.dev snapshot" }], error: null, total: 5 },
  ],
};

/** @type {GithubListing} */
const ASKING = { state: "asking", repository: null, sections: [], at: 0 };

/** @param {Partial<OpenItem>} [fields] @returns {OpenItem} */
function asking(fields = {}) {
  const subject = { number: 235, pull_request: true, repository: null, url: "https://github.com/nora-quinn/luxe-dev/pull/235" };
  return { subject, tab: "overview", overview: { state: "asking" }, conversation: { state: "asking" }, ...fields };
}

// The break this catches: the answer not kept, or kept while nothing was asked, or a reconnect
// that leaves the tab saying it's loading for good
test("the listing takes the answer to what it asked", () => {
  const listed = applyListed(ASKING, { kind: "github_listed", content: LISTED });
  assert.equal(listed.state, "listed");
  assert.equal(listed.repository, "nora-quinn/luxe-dev");
  assert.equal(listed.sections.length, 2);
  assert.ok(listed.at > 1_700_000_000, "stamped with the clock its ages are drawn from");
  const idle = { ...ASKING, state: /** @type {const} */ ("idle") };
  assert.equal(applyListed(idle, { kind: "github_listed", content: LISTED }), idle);
  assert.equal(applyListed(ASKING, { kind: "ready", content: {} }).state, "lost");
  assert.equal(applyListed(ASKING, { kind: "workspace_updated", content: {} }), ASKING);
});

// The break this catches: an answer for another item, or the same number in another repository,
// taken as this one's
test("an item takes only its own answers", () => {
  const item = asking();
  const overview = { kind: "pull_request", overview: { number: 235 } };
  assert.equal(applyItem(item, { kind: "github_overview_read", content: { number: 211, repository: null, overview } }), item);
  assert.equal(applyItem(item, { kind: "github_overview_read", content: { number: 235, repository: "a/b", overview } }), item);
  const read = applyItem(item, { kind: "github_overview_read", content: { number: 235, repository: null, overview, error: null } });
  assert.equal(read?.overview.state, "read");
  assert.deepEqual(read?.overview.value, overview);
  assert.equal(read?.conversation.state, "asking", "the overview's answer isn't the conversation's");
});

// The break this catches: a failure shown as an empty read, the conversation's answer dropped,
// or a later answer overwriting the first
test("each half is read or failed on its own", () => {
  const entries = [{ kind: "opening" }];
  const content = { number: 235, repository: null, entries, error: null };
  const read = applyItem(asking(), { kind: "github_conversation_read", content });
  assert.deepEqual(read?.conversation.value, entries);
  const failed = applyItem(asking(), { kind: "github_overview_read", content: { number: 235, repository: null, overview: null, error: "gh failed" } });
  assert.deepEqual([failed?.overview.state, failed?.overview.error], ["failed", "gh failed"]);
  const again = applyItem(read, { kind: "github_conversation_read", content: { ...content, entries: [] } });
  assert.deepEqual(again?.conversation.value, entries);
  const silent = applyItem(asking(), { kind: "github_conversation_read", content: { ...content, entries: null, error: "gone" } });
  assert.deepEqual([silent?.conversation.state, silent?.conversation.error], ["failed", "gone"]);
  const overviewRead = applyItem(asking(), { kind: "github_overview_read", content: { number: 235, repository: null, overview: 1 } });
  const overviewAgain = applyItem(overviewRead, { kind: "github_overview_read", content: { number: 235, repository: null, overview: 2 } });
  assert.equal(overviewAgain?.overview.value, 1);
});

// The break this catches: a reconnect that strands a half still asking, or throws away one
// already read
test("a reconnect loses only what was still being asked", () => {
  const item = asking({ overview: { state: "read", value: {}, at: 1 } });
  const after = applyItem(item, { kind: "ready", content: {} });
  assert.equal(after?.overview.state, "read");
  assert.equal(after?.conversation.state, "lost");
  assert.equal(applyItem(null, { kind: "ready", content: {} }), null);
});

// The break this catches: opening an item that asks for only one half, asks without saying which,
// or claims to be loading when nothing went out
test("opening an item asks for both halves", () => {
  const github = createGithub();
  /** @type {any[]} */
  const sent = [];
  github.read(asking().subject, (frame) => sent.push(frame) > 0);
  const content = { number: 235, pull_request: true, repository: null };
  assert.deepEqual(sent, [{ kind: "read_github_overview", content }, { kind: "read_github_conversation", content }]);
  assert.equal(github.open.value?.overview.state, "asking");
  github.read(asking().subject, () => false);
  assert.equal(github.open.value?.conversation.state, "lost");
});

// The break this catches: the tab never asking for its sections, or saying it's loading when
// nothing went out
test("listing asks for every section", () => {
  const github = createGithub();
  /** @type {any[]} */
  const sent = [];
  github.list((frame) => sent.push(frame) > 0);
  assert.deepEqual(sent, [{ kind: "list_github", content: {} }]);
  assert.equal(github.listing.value.state, "asking");
  github.list(() => false);
  assert.equal(github.listing.value.state, "lost");
});

// The break this catches: the tab opening on an empty section when another has rows, or losing
// the reader's pick on a refresh
test("the tab shows the pick, else the first section with rows", () => {
  assert.equal(shownSection(LISTED.sections, "Review requested").name, "Review requested");
  assert.equal(shownSection(LISTED.sections, null).name, "Pull requests");
  assert.equal(shownSection(LISTED.sections, "Gone").name, "Pull requests");
  assert.equal(shownSection([LISTED.sections[0]], null).name, "Review requested");
});

/** A row as the host draws it. @param {string} repository */
const row = (repository) => /** @type {any} */ ({ dataset: { number: "211", pull: "true", repository, url: "https://github.com/x/pull/211" } });

// The break this catches: the number read as text, the kind inverted, or the checkout's own
// repository named in the ask, which the host reads as somewhere else
test("a row says which item to open", () => {
  const here = "nora-quinn/luxe-dev";
  assert.deepEqual(rowSubject(row(here), here), { number: 211, pull_request: true, repository: null, url: "https://github.com/x/pull/211" });
  assert.equal(rowSubject(row("someone/else"), here).repository, "someone/else");
  const issue = row(here);
  issue.dataset.pull = "false";
  assert.equal(rowSubject(issue, here).pull_request, false);
});

// The break this catches: a mention that names the wrong kind, leaves out the repository the
// agent needs to read it, or drops the link
test("a mention says what it is and where", () => {
  const subject = { number: 7, pull_request: true, repository: null, url: "https://github.com/o/r/pull/7" };
  assert.equal(githubMention(subject, "o/r"), "pull request o/r#7 (https://github.com/o/r/pull/7)");
  const issue = { number: 8, pull_request: false, repository: "x/y", url: "https://github.com/x/y/issues/8" };
  assert.equal(githubMention(issue, "o/r"), "issue x/y#8 (https://github.com/x/y/issues/8)");
});

/** An event that landed in `element` when asked for `selector`. @param {string} selector @param {any} element */
const landing = (selector, element) => /** @type {any} */ ({ target: { closest: (/** @type {string} */ asked) => (asked === selector ? element : null) } });

/** Hooks with a GitHub state and what was sent. */
function hooks() {
  const github = createGithub();
  github.listing.value = { state: "listed", repository: "o/r", sections: [], at: 1 };
  /** @type {any[]} */
  const sent = [];
  const send = (/** @type {any} */ frame) => sent.push(frame) > 0;
  return { github, send, sent };
}

// The break this catches: a row tap that opens nothing, or reacts to a tap between rows
test("tapping a row opens it", () => {
  const at = hooks();
  tapGithubRow(landing("button.gh-row", row("o/r")), /** @type {any} */ (at));
  assert.equal(at.github.open.value?.subject.number, 211);
  const missed = hooks();
  tapGithubRow(landing("nothing", null), /** @type {any} */ (missed));
  assert.equal(missed.github.open.value, null);
  assert.equal(missed.sent.length, 0);
});

// The break this catches: "open #N" opening an issue or another repository's number, "more lines"
// not switching to Conversation, or "Show all" not unfolding its own card
test("taps inside the detail open, switch and unfold", () => {
  const at = hooks();
  at.github.read({ number: 235, pull_request: true, repository: "x/y", url: "" }, at.send);
  tapDetail(landing("button.gh-open", { dataset: { number: "230" } }), /** @type {any} */ (at));
  assert.deepEqual(at.github.open.value?.subject, { number: 230, pull_request: true, repository: "x/y", url: "https://github.com/x/y/pull/230" });
  tapDetail(landing("button.gh-more", {}), /** @type {any} */ (at));
  assert.equal(at.github.open.value?.tab, "conversation");
  const removed = /** @type {string[]} */ ([]);
  const card = { classList: { remove: (/** @type {string} */ name) => removed.push(name) } };
  tapDetail(landing("button.gh-unfold", { closest: () => card }), /** @type {any} */ (at));
  assert.deepEqual(removed, ["folded"]);
});

// The break this catches: a linked pull request's link pointing at nowhere when it's in the
// checkout's own repository
test("a linked pull request's link names the checkout when it's there", () => {
  const subject = { number: 1, pull_request: false, repository: null, url: "" };
  assert.equal(linkedSubject(subject, 5, "o/r").url, "https://github.com/o/r/pull/5");
  assert.equal(linkedSubject(subject, 5, "o/r").repository, null);
});

/** A section of three rows, `a` to `c`. */
const SECTION = { name: "Mine", query: "is:pr author:@me", total: 9, items: ["a", "b", "c"].map((url) => ({ url })) };
const at = (/** @type {string} */ url) => /** @type {any} */ ({ url });

// The break this catches: the arrows running off the list's ends, skipping a row, or doing nothing
// with no row open where the TUI's cursor starts on the first
test("the arrows step the open row, held at the ends", () => {
  assert.equal(steppedItem(SECTION, at("a"), 1)?.url, "b");
  assert.equal(steppedItem(SECTION, at("c"), 1)?.url, "c");
  assert.equal(steppedItem(SECTION, at("a"), -1)?.url, "a");
  assert.equal(steppedItem(SECTION, null, 1)?.url, "a");
  assert.equal(steppedItem({ items: [] }, null, 1), null);
});

// The break this catches: tab stopping at the last section where the TUI's goes round
test("tab steps the sections round the ends", () => {
  const sections = [{ name: "x" }, { name: "y" }];
  assert.equal(steppedSection(sections, "y", 1).name, "x");
  assert.equal(steppedSection(sections, "x", -1).name, "y");
});

// The break this catches: a tab counting the rows fetched rather than what the search matches, or
// its search lost, which picks its mark
test("a tab carries its search and the search's count", () => {
  assert.deepEqual(tabsOf([SECTION]), [{ name: "Mine", query: "is:pr author:@me", count: 9 }]);
  assert.deepEqual(tabsOf([{ name: "Old", items: [{}] }]), [{ name: "Old", query: null, count: 1 }]);
});

// The break this catches: the hint line's place off by one, or shown for a row not in the tab
test("the place counts from one, and is empty off the list", () => {
  assert.equal(placeOf(SECTION, at("b")), "2/3");
  assert.equal(placeOf(SECTION, at("z")), "");
});

// The break this catches: Review offered on an issue, or on another repository's pull request,
// which the checkout can't diff
test("only the checkout's own pull requests can be reviewed", () => {
  const pull = { number: 1, pull_request: true, repository: null, url: "" };
  assert.equal(reviewable(pull), true);
  assert.equal(reviewable({ ...pull, pull_request: false }), false);
  assert.equal(reviewable({ ...pull, repository: "x/y" }), false);
});
