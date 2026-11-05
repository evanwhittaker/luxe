// The app end to end in WebKit, Safari's engine: pair by pasting the invitation link, open the
// session from the list, send a message, grant the write it asks for, and watch the mock model's
// turn arrive as the TUI would show it: the rendered answer, the changed file in the Files tree,
// and its diff.
//
// Needs a built `luxe` (`LUXE_BIN`, or target/debug/luxe) and Playwright's WebKit
// (`npx playwright install webkit`). Run: npm run e2e

import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { basename } from "node:path";
import { after, before, test } from "node:test";
import { LOGIN, startHost } from "./host.mjs";
import { PICTURE, WRITTEN, startMockProvider } from "./mock-provider.mjs";

const playwright = createRequire(import.meta.url)("playwright");

let model, host, browser, page;
const troubles = [];

before(async () => {
  model = await startMockProvider();
  host = await startHost(model.port);
  browser = await playwright.webkit.launch();
  const context = await browser.newContext({
    ...playwright.devices["iPad Pro 11 landscape"],
    extraHTTPHeaders: { "Tailscale-User-Login": LOGIN },
  });
  page = await context.newPage();
  page.on("pageerror", (error) => troubles.push(`page error: ${error.message}`));
  page.on("console", (message) => message.type() === "error" && troubles.push(`console: ${message.text()}`));
});

after(async () => {
  await browser?.close();
  await host?.stop();
  await model?.close();
});

// The break this catches: any link in the chain from invitation to diff (the unpaired app's
// pairing field, pairing, the session list, the socket, the composer, the permission card,
// rendering, the Files tree, the diff request) broken in Safari's engine
test("a paired iPad drives a turn and reads its diff", async () => {
  await pair();
  await page.locator("a.session").click();
  await page.locator(".composer textarea").fill("Write the file, please.");
  await page.locator(".send").click();
  await page.locator(".prompt .choice.once").click();
  await page.locator(".item.reply strong", { hasText: WRITTEN.path }).waitFor();
  await openWrittenDiff();
  const added = page.locator(".sheet .dl-addition code");
  assert.equal((await added.textContent())?.trim(), WRITTEN.content.trim());
  assert.deepEqual(troubles, [], "the page ran without errors or refusals");
});

// The break this catches: a tool card's code drawn plain, the piece asked for under a name the
// host doesn't know, or highlighted for a language other than the written file's
test("a write's card colors its code as the file's language", async () => {
  const keyword = page.locator(".item.tool .tool-body .tool-code .t-accent-secondary", { hasText: "const" });
  await keyword.first().waitFor();
});

// Pairs as an app on the home screen would: the unpaired app takes the pasted link to the
// invitation page. Serve puts HTTPS in front of the bridge, so the credential is a Secure cookie,
// and WebKit drops one that arrives over this test's plain HTTP; the test carries it over itself,
// then opens the app as the page's redirect would.
async function pair() {
  await page.goto(host.url);
  await page.locator(".pair-link input").fill(`https://desk.example.ts.net/pair#${host.invitation}`);
  await page.locator(".pair-link button").click();
  const answered = page.waitForResponse("**/api/pair");
  await page.locator("#pair button").click();
  const cookie = (await (await answered).headerValue("set-cookie"))?.split(";")[0] ?? "";
  // let the page's own redirect land and finish asking, or the visit below races it
  await page.waitForURL((url) => url.pathname === "/", { waitUntil: "networkidle" });
  const at = cookie.indexOf("=");
  await page.context().addCookies([{ name: cookie.slice(0, at), value: cookie.slice(at + 1), url: host.url }]);
  // the page's own redirect may have asked for the list before the cookie was carried over
  troubles.length = 0;
  await page.goto(host.url);
}

// The break this catches: the turn reaching the model without the tool's result, so the answer
// on screen came from somewhere other than the loop the app drove
test("the model saw the message and the tool's result", () => {
  const turns = model.requests.filter((request) => request.tools?.length);
  assert.equal(turns.length, 2, "one request to act, one with the result");
  const sent = JSON.stringify(turns[0].messages);
  assert.ok(sent.includes("Write the file, please."), sent);
  assert.equal(turns[1].messages.at(-1).role, "tool");
});

// The break this catches: the page scrolling as a whole under a pinned bar and composer, which on
// iOS shows the transcript past the composer once the visible area drifts from the page's, as it
// does around the keyboard; or the transcript no longer following its end once it's what scrolls
test("only the transcript scrolls, and it follows new output", async () => {
  await page.keyboard.press("Escape");
  const filler = await page.evaluateHandle(() => {
    const tall = Object.assign(document.createElement("div"), { style: "height: 300vh" });
    return document.querySelector(".transcript")?.appendChild(tall);
  });
  await page.waitForFunction(() => {
    const main = /** @type {HTMLElement} */ (document.querySelector("main"));
    return main.scrollTop + main.clientHeight >= main.scrollHeight - 48;
  });
  const page_ = await page.evaluate(() => ({
    height: document.scrollingElement?.scrollHeight ?? 0,
    foot: document.querySelector(".controls")?.getBoundingClientRect().bottom ?? Infinity,
    screen: innerHeight,
  }));
  // the scroll the removal clamps lands a frame later; let it, or WebKit delivers it after the next
  // test's filler is in and the app reads a jump up from the end as the reader leaving it
  await filler.evaluate((tall) => {
    tall?.remove();
    return new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
  assert.ok(page_.height <= page_.screen, `the page is one screen: ${JSON.stringify(page_)}`);
  assert.ok(page_.foot <= page_.screen, `the composer is the screen's foot: ${JSON.stringify(page_)}`);
});

// The break this catches: what you're reading jumping when something above it changes height, as a
// reply does when its rendered HTML replaces its plain text, which WebKit doesn't anchor for
test("scrolled back, the item being read holds still as one above it grows", async () => {
  const filler = await page.evaluateHandle(() => {
    const tall = Object.assign(document.createElement("div"), { className: "item", style: "height: 300vh" });
    return document.querySelector(".transcript")?.appendChild(tall);
  });
  // at the end first, as a reader is, so the scroll below is one up and away from it
  await page.waitForFunction(() => {
    const main = /** @type {HTMLElement} */ (document.querySelector("main"));
    return main.scrollTop + main.clientHeight >= main.scrollHeight - 48;
  });
  const reading = await page.evaluate(async () => {
    const main = /** @type {HTMLElement} */ (document.querySelector("main"));
    const items = [...document.querySelectorAll(".transcript > .item")];
    const frames = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    // the scroll to the end lands as an event a frame after the page is there; scrolling up
    // before it would read as a scroll down from where the page was before the filler
    await frames();
    // the second item just under the top, so the first is wholly above it
    main.scrollTop += items[1].getBoundingClientRect().top - main.getBoundingClientRect().top - 4;
    await frames();
    const before = items[1].getBoundingClientRect().top;
    /** @type {HTMLElement} */ (items[0]).style.paddingTop = "200px";
    await frames();
    const after = items[1].getBoundingClientRect().top;
    /** @type {HTMLElement} */ (items[0]).style.paddingTop = "";
    main.scrollTop = main.scrollHeight;
    await frames();
    return { before, after };
  });
  await filler.evaluate((tall) => tall?.remove());
  assert.ok(Math.abs(reading.after - reading.before) < 2, `held in place: ${JSON.stringify(reading)}`);
});

// The break this catches: the Files part's chain (the rail, `list_directory` and
// `read_project_file` through the bridge, the file piece's rendering, Mention into the composer)
// broken in Safari's engine, or the iPad's header keeping a second button for the project
test("the rail lists the project, shows a file and mentions it", async () => {
  await page.keyboard.press("Escape");
  // The break this catches: the session's header naming its folder bare, without the branch it's on
  const header = page.locator("header.bar:has(.back) .title .project-name");
  await header.locator(".project-branch").waitFor();
  assert.match((await header.textContent()) ?? "", new RegExp(`${basename(host.project)}.*\\S+ \\*$`));
  assert.equal(await page.locator(".panel-button").count(), 0, "one header button on a wide screen");
  // The break this catches: a PLAN drawn before there's a plan, the parts out of the dock's order,
  // or MODEL not drawn by the host from the status line's facts
  await page.locator('.part[data-part="model"] .badge', { hasText: "MODEL" }).waitFor();
  const parts = await page.$$eval(".panel .part", (all) => all.map((part) => part.getAttribute("data-part")));
  assert.deepEqual(parts.slice(0, 3), ["files", "model", "session"], "no plan yet, then the dock's order");
  assert.match((await page.locator('.part[data-part="model"] .telemetry').textContent()) ?? "", /model.*mode.*build/s);
  // the file the turn wrote wears its change, as it does in the TUI's tree
  await page.locator(`button.tree-row[data-file="${WRITTEN.path}"] .tree-name[class*="change-"]`).waitFor();
  await page.locator("button.tree-row", { hasText: "README.md" }).click();
  assert.equal((await page.locator(".peek .file .fl code").first().textContent())?.trim(), "# e2e");
  // Escape closes the file and leaves the tree, as the TUI's Escape leaves a peek
  await page.keyboard.press("Escape");
  await page.locator(".peek").waitFor({ state: "detached" });
  await page.locator("button.tree-row", { hasText: "README.md" }).click();
  await page.locator(".peek .mention").click();
  await page.waitForFunction(() => document.querySelector("textarea")?.value === "`README.md` ");
  assert.equal(await page.locator(".peek").count(), 0, "Mention leaves the composer in view");
  assert.equal(await page.locator(".panel").count(), 1, "and the rail beside it");
  // Escape is for what opens over the conversation; the rail beside it stays
  await page.keyboard.press("Escape");
  assert.equal(await page.locator(".panel").count(), 1, "Escape leaves the rail");
  assert.deepEqual(troubles, [], "the page ran without errors or refusals");
});

/** Where `selector`'s first box starts, top to bottom. */
async function topOf(/** @type {string} */ selector) {
  return page.locator(selector).first().evaluate((element) => element.getBoundingClientRect().top);
}

// The break this catches: the rail's first heading and the conversation bar's title set by
// different rules, so the two sit at different heights side by side, at any of the text sizes
test("the rail's first heading and the conversation's title share a line", async () => {
  await page.setViewportSize({ width: 951, height: 669 });
  await page.locator(".panel .part").first().waitFor();
  const chosen = await page.evaluate(() => document.documentElement.dataset.textSize);
  for (const size of ["small", "medium", "large"]) {
    await page.evaluate((size) => (document.documentElement.dataset.textSize = size), size);
    const heading = await topOf(".panel .part .badge");
    const title = await topOf("header.bar:has(.back) .title");
    assert.ok(Math.abs(heading - title) < 0.5, `${size} text: the heading starts at ${heading}px, the title at ${title}px`);
  }
  await page.evaluate((size) => (document.documentElement.dataset.textSize = size), chosen);
  await page.setViewportSize(playwright.devices["iPad Pro 11 landscape"].viewport);
});

/** `text` as the host renders a reply, drawn into the transcript as one. */
async function showReply(/** @type {string} */ text) {
  await page.evaluate(async (text) => {
    const response = await fetch("/api/render", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ pieces: [{ kind: "answer", text }] }),
    });
    const reply = Object.assign(document.createElement("div"), { className: "item reply md shown-reply" });
    reply.innerHTML = (await response.json()).html[0];
    document.querySelector(".transcript")?.append(reply);
  }, text);
}

// The break this catches: a quote drawn with the terminal's `>` marks, on itself or on any block
// in it, rather than set off as markdown sets one; or set off by nothing a reader can see, with no
// rule down its side or no plate behind it
test("a quote is set off by a rule and a plate, with no marks", async () => {
  await showReply("> ### What\n>\n> Three commits, best reviewed one at a time.\n>\n> 1. one\n> 2. two");
  const quote = await page.locator(".shown-reply blockquote").evaluate((element) => {
    const style = getComputedStyle(element);
    const marked = [element, ...element.querySelectorAll("*")]
      .map((part) => getComputedStyle(part, "::before").content)
      .filter((content) => content.includes(">"));
    const reply = getComputedStyle(/** @type {Element} */ (element.parentElement)).backgroundColor;
    return { marked, rule: style.borderInlineStartWidth, ruled: style.borderInlineStartStyle, plate: style.backgroundColor, reply };
  });
  await page.locator(".shown-reply").evaluate((element) => element.remove());
  assert.deepEqual(quote.marked, [], "no `>` drawn anywhere in the quote");
  assert.equal(quote.ruled, "solid", `a rule down its side: ${JSON.stringify(quote)}`);
  assert.notEqual(quote.rule, "0px", `a rule down its side: ${JSON.stringify(quote)}`);
  assert.notEqual(quote.plate, quote.reply, `a plate behind it: ${JSON.stringify(quote)}`);
});

/**
 * Opens the turn's file from the Files tree once it wears its change, so the tap opens its diff
 * rather than reading it as it stands.
 */
async function openWrittenDiff() {
  const row = page.locator(`.panel button.tree-row[data-file="${WRITTEN.path}"]`);
  await row.locator('.tree-name[class*="change-"]').waitFor();
  await row.click();
}

/** Opens the Review view by the header's button, its guide drawn. */
async function openReview() {
  await page.locator('header.bar .view-button[aria-label="Review changes"]').click();
  await page.locator('.view-review[role="dialog"]').waitFor();
}

/** Closes the open view by Escape, as the TUI's Escape leaves the desk. */
async function closeView() {
  await page.keyboard.press("Escape");
  await page.locator(".view").waitFor({ state: "detached" });
}

// The break this catches: the Review view's chain (the header's button, `show_open_review` through
// the engine to the review desk, `load_guided_review` of the working changes, the guide and
// chapter pieces side by side on an iPad, and `record_review_progress` kept in the session the
// next ask reads, here by `/diff`) broken in Safari's engine
test("the review view reviews the changes and keeps a chapter marked", async () => {
  await openReview();
  await page.locator('.view-review button.retry:has-text("Review changes")').click();
  await page.locator(".rv-target", { hasText: "Working changes" }).waitFor();
  // split, the first chapter opens beside the guide, its title on the cursor's slab
  await page.locator(".rv-chapter-pane .rv-chapter-view .rv-file", { hasText: WRITTEN.path }).waitFor();
  await page.locator(".rv-guide-pane .rv-chapter[aria-current]").waitFor();
  await page.locator(".view-review .hint", { hasText: "mark reviewed" }).click();
  await page.locator(".view-review .hint.hint-accent", { hasText: "reviewed" }).waitFor();
  await closeView();
  await runCommand("/diff");
  await page.locator(".view-review .rv-chapter .rv-walked").first().waitFor();
  await closeView();
  assert.deepEqual(troubles, [], "the page ran without errors or refusals");
});

// The break this catches: `/github` sent to the agent rather than opening the view, the frame not
// titled as the TUI's, or the view's close hint not closing it
test("github opens its own view, framed and titled as the TUI's", async () => {
  await runCommand("/github");
  await page.locator('.view-github[role="dialog"] .view-frame.framed').waitFor();
  assert.match((await page.locator(".view-github .view-title").textContent()) ?? "", /GitHub/);
  await page.locator(".view-github .hint", { hasText: "close" }).click();
  await page.locator(".view").waitFor({ state: "detached" });
  assert.deepEqual(troubles, [], "the page ran without errors or refusals");
});

/** Opens README.md from the rail's Files part, its lines drawn. */
async function openReadme() {
  await page.locator("button.tree-row", { hasText: "README.md" }).click();
  await page.locator('.peek .fl[data-line="4"]').waitFor();
}

/** Selects `words` inside line `line` of the open file, as a long press and drag would. */
async function selectWithin(/** @type {number} */ line, /** @type {string} */ words) {
  await page.evaluate(([line, words]) => {
    const code = document.querySelector(`.peek .fl[data-line="${line}"] code`);
    const walker = document.createTreeWalker(/** @type {Node} */ (code), NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const at = node.textContent?.indexOf(String(words)) ?? -1;
      if (at < 0) continue;
      const range = document.createRange();
      range.setStart(node, at);
      range.setEnd(node, at + String(words).length);
      document.getSelection()?.removeAllRanges();
      document.getSelection()?.addRange(range);
      return;
    }
  }, [line, words]);
}

// The break this catches: line picks lost between the host's markup and the composer (rows not
// naming their lines, a tap on a number not picking, a second tap not stretching, picked rows not
// marked), or a selection inside a line, the phone's own way, not offering Ask for that line, or
// Ask writing other words than the TUI sends for the same lines
test("lines picked by number, or selected in part, are asked about in the TUI's words", async () => {
  await openReadme();
  const number = (/** @type {number} */ line) => page.locator(`.peek .fl[data-line="${line}"] .ln`);
  await number(3).click();
  await page.locator(".line-ask button", { hasText: "Ask about line 3" }).waitFor();
  await number(4).click();
  await page.locator(".line-ask button", { hasText: "Ask about lines 3–4" }).waitFor();
  assert.equal(await page.locator(".peek .fl.picked").count(), 2, "the picked rows are marked");
  await page.locator(".line-ask button").click();
  await page.waitForFunction(() => document.querySelector("textarea")?.value.includes("`README.md` lines 3–4"));
  assert.equal(await page.locator(".peek").count(), 0, "Ask leaves the composer in view");
  await openReadme();
  await selectWithin(3, "4519");
  await page.locator(".line-ask button", { hasText: "Ask about line 3" }).click();
  await page.waitForFunction(() => document.querySelector("textarea")?.value.includes("`README.md` line 3"));
  assert.deepEqual(troubles, [], "the page ran without errors or refusals");
});

/** Writes a comment in the open composer, as a question or not, and saves it unsent. */
async function saveComment(/** @type {string} */ words, question = false) {
  if (question) await page.locator(".rv-kinds button", { hasText: "Question" }).click();
  await page.locator(".rv-compose textarea").fill(words);
  await page.locator(".rv-compose button", { hasText: "Save" }).click();
}

// The break this catches: comments lost between the phone and the review desk (a line pick not
// anchoring to its line, the chapter comment not offered with nothing picked, the desk's saved
// session not redrawn as cards, a card's Delete not reaching the desk, or the guide not counting
// what's unsent)
test("a review chapter takes comments on its lines and on itself, kept by the host", async () => {
  await openReview();
  await page.locator(".rv-chapter .rv-title").first().click();
  await page.locator('.rv-chapter-pane .dl[data-line="1"] .ln').first().click();
  await page.locator(".line-ask button", { hasText: /^Comment$/ }).click();
  await page.locator(".rv-compose", { hasText: "Comment on line 1" }).waitFor();
  await saveComment("Why this file?", true);
  const asked = page.locator(".rv-chapter-pane .rv-finding", { hasText: "Why this file?" });
  await asked.locator(".rv-state", { hasText: "unsent" }).waitFor();
  assert.match((await asked.locator(".rv-finding-head").textContent()) ?? "", /question.*line 1/);
  await page.locator(".line-ask button", { hasText: "Comment on the chapter" }).click();
  await saveComment("Split this up.");
  await page.locator(".rv-chapter-pane .rv-finding", { hasText: "Split this up." }).waitFor();
  await asked.locator('button[data-act="delete"]').click();
  await asked.waitFor({ state: "detached" });
  await page.locator(".rv-guide-pane .rv-unsent", { hasText: "1 unsent comment" }).waitFor();
  await closeView();
  assert.deepEqual(troubles, [], "the page ran without errors or refusals");
});

/** Sends `command` from the composer, as typed. */
async function runCommand(/** @type {string} */ command) {
  await page.locator(".composer textarea").fill(command);
  await page.locator(".send").click();
}

/** Opens the phone's sidebar sheet by the header's one button. */
async function openSheet() {
  await page.locator(".panel-button").click();
  await page.locator('.panel[role="dialog"]').waitFor();
}

// The break this catches: a phone keeping two header buttons, the button not opening the sheet,
// the sheet's bar not naming the project and its branch, a diff opened from the tree drawn under
// the sheet, Escape closing the sheet with the diff or not armed the moment the sheet shows, or
// `/dock` on a phone doing nothing
test("on a phone one header button opens the sidebar", async () => {
  await page.setViewportSize({ width: 375, height: 812 });
  // no rail and no rail button, once the app sees the narrow screen
  await page.locator(".dock-toggle").waitFor({ state: "detached", timeout: 2000 });
  await page.locator(".panel").waitFor({ state: "detached", timeout: 2000 });
  await openSheet();
  const named = page.locator(".panel .bar .project-name");
  assert.ok((await named.textContent())?.includes(basename(host.project)), "the bar names the project");
  assert.ok(await named.locator(".project-mark").first().textContent(), "behind the tree's folder mark");
  // the project is a git repository the turn wrote into, so its branch shows, starred
  assert.match((await named.locator(".project-branch").textContent()) ?? "", /\S+ \*$/);
  await openWrittenDiff();
  await page.locator(".peek .dl-addition").waitFor();
  const onTop = await page.evaluate(() => document.elementFromPoint(innerWidth / 2, innerHeight / 2)?.closest(".peek") !== null);
  assert.ok(onTop, "the diff covers the panel");
  await page.keyboard.press("Escape");
  await page.locator(".peek").waitFor({ state: "detached" });
  assert.equal(await page.locator(".panel").count(), 1, "the diff took the Escape, not the panel");
  await page.keyboard.press("Escape");
  await page.locator(".panel").waitFor({ state: "detached", timeout: 2000 });
  // straight away: Escape is armed as soon as the sheet is on screen, not a frame later
  await openSheet();
  await page.keyboard.press("Escape");
  await page.locator(".panel").waitFor({ state: "detached", timeout: 2000 });
  await runCommand("/dock");
  await page.locator('.panel[role="dialog"]').waitFor();
  await page.locator(".panel .close").click();
  await page.setViewportSize(playwright.devices["iPad Pro 11 landscape"].viewport);
  assert.deepEqual(troubles, [], "the page ran without errors or refusals");
});

// The break this catches: a phone squeezing the guide and the chapter side by side, a chapter tap
// not taking the guide's place, or the hint line's way back not returning to it
test("on a phone the review shows the guide, then a chapter in its place", async () => {
  await page.setViewportSize({ width: 375, height: 812 });
  await openReview();
  assert.equal(await page.locator(".rv-chapter-pane").isVisible(), false, "the guide alone");
  await page.locator(".rv-chapter .rv-title").first().click();
  await page.locator(".rv-chapter-pane .rv-file", { hasText: WRITTEN.path }).waitFor();
  assert.equal(await page.locator(".rv-guide-pane").isVisible(), false, "the chapter in its place");
  await page.locator(".view-review .hint", { hasText: "guide" }).click();
  await page.locator(".rv-guide-pane").waitFor({ state: "visible" });
  await closeView();
  await page.setViewportSize(playwright.devices["iPad Pro 11 landscape"].viewport);
  assert.deepEqual(troubles, [], "the page ran without errors or refusals");
});

/** The middle of `selector`'s box, top to bottom. */
async function middleOf(/** @type {string} */ selector) {
  return page.locator(selector).evaluate((element) => {
    const box = element.getBoundingClientRect();
    return box.top + box.height / 2;
  });
}

// The break this catches: the mode left in the status line or the composer's foot rather than
// leading the row over the box as the TUI's does, a tap on it that switches nothing, the box not
// taking plan's color with it, or the status line wrapping on the narrowest iPhone
test("on a phone the mode leads the row over the composer and colors it", async () => {
  await page.setViewportSize({ width: 375, height: 812 });
  const mode = page.locator(".activity-row .mode");
  await mode.waitFor();
  assert.equal(await page.locator(".status-line .mode, .composer-tools .mode").count(), 0, "nowhere else");
  assert.ok((await middleOf(".activity-row .mode")) < (await middleOf(".composer textarea")), "over the box");
  await mode.click();
  await page.locator(".composer.mode-plan .activity-row .mode-plan").waitFor();
  const send = await page.locator(".send").evaluate((button) => getComputedStyle(button).color);
  await page.locator(".activity-row .mode").click();
  await page.locator(".composer.mode-build .activity-row .mode-build").waitFor();
  assert.notEqual(await page.locator(".send").evaluate((button) => getComputedStyle(button).color), send, "Send wears the mode");
  const line = await page.locator(".status-line").evaluate((element) => element.getBoundingClientRect().height);
  const cell = await page.locator(".status-line .seg").first().evaluate((element) => element.getBoundingClientRect().height);
  assert.ok(line < cell * 1.5, `the status line is one row: ${line}px for a ${cell}px cell`);
  await page.setViewportSize(playwright.devices["iPad Pro 11 landscape"].viewport);
  assert.deepEqual(troubles, [], "the page ran without errors or refusals");
});

// The break this catches: the status line's left side missing, so the bar's mirror (the change
// count the dock also keeps) never reaches the app, or drawn away from the line's left edge
test("the status line's left side counts the changed files", async () => {
  const changes = page.locator(".status-line .cluster.left .seg-changes");
  await changes.waitFor();
  assert.match((await changes.textContent()) ?? "", /\b1 file$/);
  const [line, left] = await Promise.all([".status-line", ".status-line .cluster.left"].map((selector) =>
    page.locator(selector).evaluate((element) => element.getBoundingClientRect().left)));
  assert.ok(Math.abs(line - left) < 1, `the left side rests on the edge: ${left}px against ${line}px`);
  assert.deepEqual(troubles, [], "the page ran without errors or refusals");
});

// The break this catches: an image a tool returned (a read picture, a screenshot) reaching the
// model and never the reader, though the TUI shows it under the call's card; or its bytes mangled
// on the way, so the page draws a broken image
test("a picture the agent reads shows under its card", async () => {
  await page.locator(".composer textarea").fill("Read the picture, please.");
  await page.locator(".send").click();
  await page.locator(".item.reply strong", { hasText: PICTURE.path }).waitFor();
  const picture = page.locator(".item.tool .tool-pictures img");
  await picture.waitFor();
  await page.waitForFunction(() => /** @type {HTMLImageElement | null} */ (document.querySelector(".tool-pictures img"))?.complete);
  assert.equal(await picture.evaluate((img) => /** @type {HTMLImageElement} */ (img).naturalWidth), 1, "the pixel decoded");
  assert.deepEqual(troubles, [], "the page ran without errors or refusals");
});
