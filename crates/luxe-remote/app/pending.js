// What's waiting: prompts waiting on you, and steering waiting to reach the agent.
//
// A pure fold of frames, like the transcript. Prompts come and go by id: `permission_requested`
// and `question_asked` add one, `prompt_closed` takes it down whoever answered it. A new
// connection starts over, since the session's snapshot sends every prompt still open.
//
// Steering follows the TUI's tray: a checkpoint delivers everything buffered at once, and a steer
// that misses the last one waits for the next turn, so a turn ending leaves it pending. Only a
// cancel or a withdrawal empties the buffer. What this device steered is kept as it was sent, so
// the tray can show it until it lands.

import { batch, signal } from "@preact/signals";

/**
 * @typedef {{ kind: "permission", id: string, tool: string, summary: string, always_patterns: string[] }} PermissionPrompt
 * @typedef {{ label: string, description?: string | null, recommended?: boolean }} QuestionOption
 * @typedef {{ kind: "question", id: string, prompt: string, options: QuestionOption[], multi: boolean }} QuestionPrompt
 * @typedef {PermissionPrompt | QuestionPrompt} Prompt
 * @typedef {{ text: string, photos: import("./photos.js").Photo[] }} Steer what was written and attached
 * @typedef {{ prompts: Signal<Prompt[]>, steering: Signal<Steer[]> }} State
 */
/** @import { Signal } from "@preact/signals" */
/** @import { Frame } from "./connection.js" */

/** Nothing waiting, and the functions that change that. */
export function createPending() {
  /** @type {State} */
  const state = { prompts: signal([]), steering: signal(/** @type {Steer[]} */ ([])) };
  return {
    prompts: state.prompts,
    steering: state.steering,
    /** @param {Frame} frame */
    apply: (frame) => batch(() => apply(state, frame)),
    /** This device sent `steer` into the running turn. @param {Steer} steer */
    steered: (steer) => (state.steering.value = [...state.steering.value, steer]),
    /** This device withdrew or cancelled, which empties the engine's steering buffer. */
    dropSteering: () => (state.steering.value = []),
  };
}

/** @param {State} state @param {Frame} frame */
function apply(state, { kind, content }) {
  switch (kind) {
    case "ready": state.prompts.value = []; state.steering.value = []; return;
    case "permission_requested": return open(state, { kind: "permission", ...content });
    case "question_asked": return open(state, { kind: "question", ...content });
    case "prompt_closed": return close(state, content.id);
    case "steering_delivered": state.steering.value = []; return;
  }
}

/** @param {State} state @param {Prompt} prompt */
function open(state, prompt) {
  if (state.prompts.value.some((known) => known.id === prompt.id)) return;
  state.prompts.value = [...state.prompts.value, prompt];
}

/** @param {State} state @param {string} id */
function close(state, id) {
  state.prompts.value = state.prompts.value.filter((prompt) => prompt.id !== id);
}

/**
 * The reply frame for a permission prompt.
 * @param {string} id @param {string} reply @returns {Frame}
 */
export function permissionReply(id, reply) {
  return { kind: "respond_permission", content: { id, reply } };
}

/**
 * The reply frame for a question: the chosen answers, or none for a dismissal.
 * @param {string} id @param {string[] | null} answered @returns {Frame}
 */
export function questionReply(id, answered) {
  return { kind: "answer_question", content: { id, reply: answered ? { answered } : "cancelled" } };
}

/**
 * What the composer sends: steering for the turn running, photos and all, or a new turn. The
 * photos go only when there are some, as the protocol leaves an empty list out.
 * @param {string} text @param {boolean} busy @param {{ data: string, mime: string, name: string | null }[]} images
 * @returns {Frame}
 */
export function messageFrame(text, busy, images = []) {
  const content = images.length ? { text, images } : { text };
  return { kind: busy ? "steer_turn" : "submit_user_message", content };
}
