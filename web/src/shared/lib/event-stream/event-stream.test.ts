import { describe, expect, it } from "vitest";

import { createEventParser, readEvents } from "./event-stream";

describe("event stream", () => {
  it("joins chunks split anywhere into whole events", () => {
    const parse = createEventParser();
    expect(parse("event: ready\nid: 1.1.5\nda")).toEqual([]);
    expect(parse('ta: {"resumed":false}\n\n:keep-alive\n\n')).toEqual([
      { name: "ready", id: "1.1.5", data: '{"resumed":false}' },
    ]);
  });

  it("reports an id-only event so the position still advances", () => {
    const parse = createEventParser();
    expect(parse("id: 1.1.9\n\n")).toEqual([{ name: "", id: "1.1.9", data: "" }]);
  });

  it("keeps the last id for events that do not set one", () => {
    const parse = createEventParser();
    expect(
      parse("id: 7\r\nevent: change\r\ndata: a\r\ndata: b\r\n\r\nevent: change\ndata: c\n\n"),
    ).toEqual([
      { name: "change", id: "7", data: "a\nb" },
      { name: "change", id: "7", data: "c" },
    ]);
  });

  it("reads events from a byte stream", async () => {
    const bytes = new TextEncoder().encode("event: change\nid: 2\ndata: {}\n\n");
    const body = new ReadableStream<Uint8Array<ArrayBuffer>>({
      start(controller) {
        controller.enqueue(bytes.slice(0, 5));
        controller.enqueue(bytes.slice(5));
        controller.close();
      },
    });
    const events = [];
    for await (const event of readEvents(body)) {
      events.push(event);
    }
    expect(events).toEqual([{ name: "change", id: "2", data: "{}" }]);
  });
});
