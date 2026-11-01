// The status line's facts, folded from frames, and the mode switch.

import assert from "node:assert/strict";
import { test } from "node:test";
import { activityLabel, createStatus, nextModeFrame } from "../app/status.js";

const frame = (kind, content) => ({ kind, content });
const model = { provider: "github-copilot", model: "claude-haiku-4.5", display_name: "Claude Haiku 4.5 (latest)" };

// The break this catches: each frame replacing the facts rather than adding to them, so a
// thinking change wipes the context reading until the next call reports it again
test("each frame updates its own facts and keeps the rest", () => {
  const status = createStatus();
  status.apply(frame("context_usage", { used_tokens: 9613, context_window: 200000 }));
  status.apply(frame("state_changed", { model, streaming: false, mode: "plan", thinking: "high" }));
  assert.deepEqual(status.facts.value, {
    context: { used_tokens: 9613, context_window: 200000 },
    model,
    thinking: "high",
  });
  assert.equal(status.mode.value, "plan");
});

// The break this catches: the plan's allowance or the session's spend never reaching the host,
// so the cost slot stays empty; the spend goes as its total, which is all the line shows
test("the plan and the spend are kept for the cost slot", () => {
  const status = createStatus();
  const plan = { plan: "individual", primary: { used_percent: 22.3, window_minutes: 43200 } };
  const total = { calls: 14, cost_usd: 0, unpriced_calls: 0, plan_usd: 0.045, plan_calls: 14 };
  status.apply(frame("plan_usage", plan));
  status.apply(frame("session_usage", { total, by_source: [{ source: "agent", totals: total }] }));
  assert.deepEqual(status.facts.value, { plan, session: { total } });
});

/** A transcript item as transcript.js keeps one, with plain objects for its signals. @returns {any} */
const said = (kind) => ({ key: kind, kind, text: { value: "" } });
/** @returns {any} */
const tool = (name, state) => ({ key: name, kind: "tool", card: { value: { name, state } } });

// The break this catches: a spinner left in the status line between turns
test("between turns the status line says nothing is working", () => {
  assert.equal(activityLabel(false, [said("user"), tool("bash", "running")]), null);
});

// The break this catches: the running tool not named, or a finished one named as still running
test("a turn names the tool running, and says working once it lands", () => {
  assert.equal(activityLabel(true, [said("user"), tool("read", "succeeded"), tool("bash", "running")]), "bash");
  assert.equal(activityLabel(true, [said("user"), tool("bash", "succeeded"), said("assistant")]), "working");
});

// The break this catches: a card an earlier turn left running (one replayed from a dropped link)
// named as this turn's work
test("only the turn's own items name its tool", () => {
  assert.equal(activityLabel(true, [tool("bash", "running"), said("user")]), "working");
});

// The break this catches: a `/compact` between turns, a minute with nothing streaming, showing no
// sign anything is happening; or a mid-turn one named as the tool that was running before it
test("a summary being written says compacting, in a turn or between them", () => {
  assert.equal(activityLabel(false, [], true), "compacting");
  assert.equal(activityLabel(true, [said("user"), tool("bash", "running")], true), "compacting");
});

// The break this catches: the mode switch sending the mode already on, so a tap does nothing
test("a tap on the mode switches to the other one", () => {
  assert.deepEqual(nextModeFrame("build"), { kind: "set_mode", content: "plan" });
  assert.deepEqual(nextModeFrame("plan"), { kind: "set_mode", content: "build" });
});
