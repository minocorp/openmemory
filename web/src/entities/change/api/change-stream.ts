import { client, isApiError, unwrap } from "@/shared/api";
import { readEvents } from "@/shared/lib/event-stream";

import {
  changeEventSchema,
  streamReadySchema,
  type ChangeEvent,
  type StreamReady,
} from "../model/change-event";

const firstRetryDelayMs = 1_000;
const maxRetryDelayMs = 30_000;

export type ChangeHandlers = {
  onReady: (ready: StreamReady) => void;
  onChange: (change: ChangeEvent) => void;
};

function openChangeStream(cursor: string | null, signal: AbortSignal) {
  return unwrap(
    client.GET("/api/changes/stream", {
      params: { header: cursor === null ? {} : { "Last-Event-ID": cursor } },
      parseAs: "stream",
      signal,
    }),
  );
}

function isTransient(error: unknown): boolean {
  return error instanceof TypeError || (isApiError(error) && error.status >= 500);
}

async function wait(delayMs: number, signal: AbortSignal) {
  await new Promise<void>((resolve) => {
    const stop = () => {
      clearTimeout(timer);
      resolve();
    };
    const timer = setTimeout(() => {
      signal.removeEventListener("abort", stop);
      resolve();
    }, delayMs);
    signal.addEventListener("abort", stop, { once: true });
  });
}

export async function followChanges(handlers: ChangeHandlers, signal: AbortSignal) {
  let cursor: string | null = null;
  let retryDelayMs = firstRetryDelayMs;
  while (!signal.aborted) {
    try {
      const body = await openChangeStream(cursor, signal);
      for await (const event of body === null ? [] : readEvents(body)) {
        cursor = event.id;
        if (event.name === "ready") {
          retryDelayMs = firstRetryDelayMs;
          handlers.onReady(streamReadySchema.parse(JSON.parse(event.data)));
        } else if (event.name === "change") {
          handlers.onChange(changeEventSchema.parse(JSON.parse(event.data)));
        }
      }
    } catch (error) {
      if (signal.aborted) {
        return;
      }
      if (!isTransient(error)) {
        throw error;
      }
    }
    await wait(retryDelayMs, signal);
    retryDelayMs = Math.min(retryDelayMs * 2, maxRetryDelayMs);
  }
}
