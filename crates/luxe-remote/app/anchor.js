// @ts-check
// Keeps what you're reading still while the transcript changes above it. Scrolled back, the page
// holds its place on the first item showing at the top: when anything above it changes height (a
// reply's rendered HTML replacing its plain text, a picture loading, "Show earlier" adding a page
// of history) the scroll moves by just as much. Chrome and Firefox do this themselves, as scroll
// anchoring; WebKit doesn't, which is the jumping an iPhone or iPad showed. So the browsers' own
// anchoring is off (style.css) and this does it everywhere, one way. Following the end is
// `followGrowth`'s job, and this stands aside while it does.

/** @import { Signal } from "@preact/signals" */
/** @typedef {{ item: Element, offset: number }} Anchor */

/**
 * Holds `main`'s place on its first showing item while `following` is off. Returns the cleanup.
 * @param {HTMLElement} main @param {Signal<boolean>} following
 */
export function holdReadingPlace(main, following) {
  const list = main.firstElementChild;
  if (!list) return () => {};
  /** @type {Anchor | null} */
  let anchor = null;
  const remember = () => (anchor = anchorIn(list, main.getBoundingClientRect().top));
  const settle = () => {
    if (!following.value && anchor) main.scrollTop += anchorShift(anchor, main.getBoundingClientRect().top);
    remember();
  };
  const watcher = new ResizeObserver(settle);
  watcher.observe(list);
  main.addEventListener("scroll", remember, { passive: true });
  remember();
  return () => {
    watcher.disconnect();
    main.removeEventListener("scroll", remember);
  };
}

/**
 * The first item in `list` still showing below `top`, and how far below `top` it starts; never the
 * "Show earlier" button, which stays first while what it adds goes in under it.
 * @param {Element} list @param {number} top @returns {Anchor | null}
 */
export function anchorIn(list, top) {
  for (const item of list.children) {
    if (item.classList.contains("earlier")) continue;
    const box = item.getBoundingClientRect();
    if (box.bottom > top) return { item, offset: box.top - top };
  }
  return null;
}

/**
 * How far `anchor` has moved since it was taken, which is how far to scroll to put it back; none
 * once it's left the page.
 * @param {Anchor} anchor @param {number} top
 */
export function anchorShift(anchor, top) {
  if (!anchor.item.isConnected) return 0;
  return anchor.item.getBoundingClientRect().top - top - anchor.offset;
}
