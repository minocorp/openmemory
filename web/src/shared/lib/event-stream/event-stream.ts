export type StreamEvent = { name: string; id: string; data: string };

type Block = { name: string; data: string[]; hasId: boolean };

const emptyBlock: Block = { name: "", data: [], hasId: false };

export function createEventParser(): (chunk: string) => StreamEvent[] {
  let buffer = "";
  let id = "";
  let block = emptyBlock;

  const readLine = (line: string): StreamEvent | null => {
    if (line === "") {
      const finished = block;
      block = emptyBlock;
      if (finished.data.length === 0 && !finished.hasId) {
        return null;
      }
      return { name: finished.name, id, data: finished.data.join("\n") };
    }
    const colon = line.indexOf(":");
    const field = colon === -1 ? line : line.slice(0, colon);
    const value = colon === -1 ? "" : line.slice(colon + 1).replace(/^ /, "");
    if (field === "event") {
      block = { ...block, name: value };
    } else if (field === "data") {
      block = { ...block, data: [...block.data, value] };
    } else if (field === "id") {
      id = value;
      block = { ...block, hasId: true };
    }
    return null;
  };

  return (chunk) => {
    const lines = (buffer + chunk).split(/\r\n|\r|\n/);
    buffer = lines.pop() ?? "";
    return lines.map(readLine).filter((event) => event !== null);
  };
}

export async function* readEvents(
  body: ReadableStream<Uint8Array<ArrayBuffer>>,
): AsyncGenerator<StreamEvent, void> {
  const parse = createEventParser();
  const reader = body.pipeThrough(new TextDecoderStream()).getReader();
  try {
    let result = await reader.read();
    while (!result.done) {
      yield* parse(result.value);
      result = await reader.read();
    }
  } finally {
    reader.releaseLock();
  }
}
