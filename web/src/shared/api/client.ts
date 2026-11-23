import createClient from "openapi-fetch";

import type { paths } from "./schema";

const safeMethods = new Set(["GET", "HEAD", "OPTIONS"]);

export const client = createClient<paths>({ baseUrl: globalThis.location?.origin ?? "" });

// The server rejects any unsafe request without a JSON content type, even one with no body
client.use({
  onRequest: ({ request }) => {
    if (!safeMethods.has(request.method) && !request.headers.has("Content-Type")) {
      request.headers.set("Content-Type", "application/json");
    }
    return request;
  },
});
