// The composer foot's model and thinking lists.

import assert from "node:assert/strict";
import { test } from "node:test";
import { emptyNote, quickChoices } from "../app/quick-pick.js";

const haiku = { provider: "anthropic", model: "haiku", name: "Haiku", thinking_levels: ["off", "high"] };
const sonnet = { provider: "anthropic", model: "sonnet", name: "Sonnet", thinking_levels: ["low"] };
/** @type {import("../app/models.js").Listing} */
const listing = { state: "listed", models: [haiku, sonnet] };
const facts = { model: { provider: "anthropic", model: "haiku" }, thinking: "high" };

// The break this catches: the running model left unmarked (or another marked), or a pick sending
// something other than the switch to that model
test("the model list marks the running one and picks with select_model", () => {
  const choices = quickChoices("model", listing, facts);
  assert.deepEqual(choices.map((choice) => [choice.name, choice.current]), [["Haiku", true], ["Sonnet", false]]);
  assert.deepEqual(choices[1].frame, { kind: "select_model", content: { provider: "anthropic", model: "sonnet" } });
});

// The break this catches: the levels offered being some other model's, so a pick is clamped
// without a word, or the current level left unmarked
test("the thinking list is the running model's levels", () => {
  const choices = quickChoices("thinking", listing, facts);
  assert.deepEqual(choices.map((choice) => [choice.name, choice.current]), [["off", false], ["high", true]]);
  assert.deepEqual(choices[0].frame, { kind: "set_thinking_level", content: "off" });
});

// The break this catches: an older luxe that can't list models leaving the list saying it's loading
test("an empty list says what it waits on", () => {
  assert.match(emptyNote("unsupported"), /Update luxe/);
  assert.equal(emptyNote("asking"), "Loading…");
  assert.equal(emptyNote("listed"), "Nothing else to pick.");
});
