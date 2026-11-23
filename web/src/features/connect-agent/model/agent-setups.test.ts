import { describe, expect, it } from "vitest";

import { agentSetups, mcpUrl } from "./agent-setups";

describe("agent setups", () => {
  const origin = "https://memory.example.com";
  const find = (id: string) => agentSetups(origin).find((setup) => setup.id === id);

  it("points every agent at the MCP endpoint of the origin", () => {
    expect(mcpUrl(origin)).toBe("https://memory.example.com/mcp");
    expect(find("claude-code")?.snippets[0]?.text).toBe(
      "claude mcp add --transport http memory https://memory.example.com/mcp",
    );
    expect(find("codex")?.snippets[0]?.text).toContain("--url https://memory.example.com/mcp");
  });

  it("writes valid JSON for Cursor and the server origin for the CLI", () => {
    const cursor = find("cursor")?.snippets[0]?.text ?? "";
    expect(JSON.parse(cursor)).toEqual({
      mcpServers: { memory: { url: "https://memory.example.com/mcp" } },
    });
    expect(find("cli")?.snippets[0]?.text).toBe(
      "memory auth login --host https://memory.example.com",
    );
  });
});
