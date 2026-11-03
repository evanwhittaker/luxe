// A model that follows a script, served the way an OpenAI-compatible endpoint streams: asked to
// act, it calls `write`, or `read` on the project's picture when the ask names one; given the
// tool's result, it says what it did. A request with no tools (a session title, say) gets a word
// back. Every request is kept for the test to look at.

import { createServer } from "node:http";

/**
 * The file the scripted model writes, and what it writes in it: one line, in a language to color,
 * that rustfmt leaves as one line when the write formats it.
 */
export const WRITTEN = { path: "hello.rs", content: 'const HELLO: &str = "hello from the mock";\n' };

/** A picture in the host's project, one pixel, which the scripted model reads when asked to. */
export const PICTURE = {
  path: "pixel.png",
  base64: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
};

/** Starts the server on a free port. */
export async function startMockProvider() {
  const requests = [];
  const server = createServer((request, response) => answer(request, response, requests));
  await new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(undefined)));
  const { port } = /** @type {import("node:net").AddressInfo} */ (server.address());
  return { port, requests, close: () => new Promise((resolve) => server.close(resolve)) };
}

async function answer(request, response, requests) {
  if (request.method !== "POST" || !request.url?.endsWith("/chat/completions")) {
    response.writeHead(404).end();
    return;
  }
  const body = JSON.parse(await readBody(request));
  requests.push(body);
  response.writeHead(200, { "content-type": "text/event-stream" });
  for (const chunk of reply(body)) response.write(`data: ${JSON.stringify(chunk)}\n\n`);
  response.end("data: [DONE]\n\n");
}

async function readBody(request) {
  let text = "";
  for await (const part of request) text += part;
  return text;
}

/**
 * Acts on the newest ask, then says what it did once the result is in. Whether it has acted is
 * read from the history rather than the last message's role, since an image a tool returned
 * follows its `tool` message as a `user` one.
 */
function reply(body) {
  if (!body.tools?.length) return words("Mock");
  const { messages } = body;
  const asked = messages.findLastIndex(isAsk);
  const acted = messages.findLastIndex((message) => message.tool_calls?.length);
  if (acted > asked) return words(messages[acted].tool_calls[0].function.name === "read" ? `Read **${PICTURE.path}**.` : `Wrote **${WRITTEN.path}**.`);
  if (JSON.stringify(messages[asked]?.content).includes("picture")) return toolCall("read", { path: PICTURE.path });
  return toolCall("write", WRITTEN);
}

/** A message the person wrote, not the images a tool's result brought along. */
function isAsk(message) {
  return message.role === "user" && !JSON.stringify(message.content).includes("from tool result");
}

const chunk = (delta, finish = null) => ({
  id: "mock", object: "chat.completion.chunk", created: 0, model: "scripted",
  choices: [{ index: 0, delta, finish_reason: finish }],
});

const usage = { id: "mock", object: "chat.completion.chunk", created: 0, model: "scripted", choices: [],
  usage: { prompt_tokens: 1200, completion_tokens: 20, total_tokens: 1220 } };

function words(text) {
  const half = Math.ceil(text.length / 2);
  return [chunk({ role: "assistant", content: text.slice(0, half) }), chunk({ content: text.slice(half) }),
    chunk({}, "stop"), usage];
}

function toolCall(name, args_) {
  const call = { index: 0, id: `call_${name}`, type: "function", function: { name, arguments: "" } };
  const args = { index: 0, function: { arguments: JSON.stringify(args_) } };
  return [chunk({ role: "assistant", content: null, tool_calls: [call] }), chunk({ tool_calls: [args] }),
    chunk({}, "tool_calls"), usage];
}
