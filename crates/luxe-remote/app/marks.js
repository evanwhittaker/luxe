// @ts-check
// The TUI's marks on the app's own buttons, tabs and titles: the robot and the bulb the status line
// draws, `/github`'s mark, the desk's outline. Each comes from the host at the font's tier, so it
// matches the TUI's; a drawn icon stands in until the host says, or from a host with none to give.

import { html } from "./html.js";
import { Icon } from "./icons.js";
import { useRendered } from "./render.js";

/**
 * @typedef {"model" | "thinking" | "github" | "review" | "overview" | "conversation"} MarkName
 * @typedef {import("./icons.js").IconName} IconName
 */

/**
 * `mark` as the host draws it, or `drawn` meanwhile (nothing when there's none to draw).
 * @param {{ mark: MarkName, className?: string, drawn?: IconName }} props
 */
export function ChromeMark({ mark, className = "chrome-mark", drawn }) {
  const glyph = useRendered({ kind: "chrome_mark", mark }, false);
  if (glyph) return html`<span class=${className} aria-hidden="true" dangerouslySetInnerHTML=${{ __html: glyph }}></span>`;
  return drawn ? html`<${Icon} name=${drawn} />` : null;
}
