// @ts-check
// The GitHub and Review views as a session opens them: from the header's two buttons, from
// `/github` and `/diff` as the TUI's commands, and Review on a pull request in GitHub, which
// loads a review of it as Enter does in the TUI's `/github`. A mention from either closes it, so
// the draft the words went into is in sight.

import { GithubView } from "./github-view.js";
import { html } from "./html.js";
import { ChromeMark } from "./marks.js";
import { ReviewView } from "./review-view.js";
import { SPLIT } from "./views.js";

/** @import { Views, ViewName } from "./views.js" */
/** @import { Target } from "./review.js" */
/**
 * What the views work the session with.
 * @typedef {{ views: Views, github: ReturnType<typeof import("./github.js").createGithub>,
 *   review: ReturnType<typeof import("./review.js").createReview>, send: (frame: import("./connection.js").Frame) => boolean,
 *   mention: (words: string) => void, clear: () => void }} ViewHooks
 */

/** The header's way into each view, wearing the TUI's marks. @param {{ open: (name: ViewName) => void }} props */
export function ViewButtons({ open }) {
  return html`<button type="button" class="view-button" aria-label="GitHub" onClick=${() => open("github")}>
      <${ChromeMark} mark="github" className="view-mark" drawn="github" /></button>
    <button type="button" class="view-button" aria-label="Review changes" onClick=${() => open("review")}>
      <${ChromeMark} mark="review" className="view-mark" drawn="review" /></button>`;
}

/**
 * Opens view `name` over everything else, the sidebar sheet and any pane closed under it; a
 * `target` loads a review of it first. On a phone each opens on its overview, GitHub's list or
 * the desk's guide, rather than the row or chapter read last.
 * @param {ViewHooks} hooks @param {ViewName} name @param {Target} [target]
 */
export function openView({ views, github, review, send, clear }, name, target) {
  clear();
  if (target) review.load(send, target);
  if (!matchMedia(SPLIT).matches) (name === "review" ? review : github).close();
  views.open(name);
}

/** The view open, if one is. @param {{ hooks: ViewHooks }} props */
export function OpenView({ hooks }) {
  const { views, github, review, send, mention } = hooks;
  switch (views.shown.value) {
    case "github": {
      /** @param {import("./github.js").Subject} subject */
      const reviewIt = (subject) => openView(hooks, "review", { kind: "pull_request", number: subject.number });
      return html`<${GithubView} hooks=${{ github, send, mention, review: reviewIt, close: views.close }} />`;
    }
    case "review": return html`<${ReviewView} hooks=${{ review, send, mention, close: views.close }} />`;
    default: return null;
  }
}
