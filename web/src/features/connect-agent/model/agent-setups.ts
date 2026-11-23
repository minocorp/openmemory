export type AgentSetup = {
  id: string;
  label: string;
  steps: string[];
  snippets: { label: string; text: string }[];
};

export function mcpUrl(origin: string): string {
  return `${origin}/mcp`;
}

export function agentSetups(origin: string): AgentSetup[] {
  const url = mcpUrl(origin);
  return [
    {
      id: "claude-code",
      label: "Claude Code",
      steps: [],
      snippets: [{ label: "Command", text: `claude mcp add --transport http memory ${url}` }],
    },
    {
      id: "claude",
      label: "claude.ai and Claude Desktop",
      steps: ["Customize, Connectors, Add custom connector", "Paste the URL"],
      snippets: [{ label: "URL", text: url }],
    },
    {
      id: "chatgpt",
      label: "ChatGPT",
      steps: ["Plugins, Add custom MCP server", "Paste the URL, choose OAuth"],
      snippets: [{ label: "URL", text: url }],
    },
    {
      id: "codex",
      label: "Codex",
      steps: [],
      snippets: [
        { label: "Commands", text: `codex mcp add memory --url ${url}\ncodex mcp login memory` },
      ],
    },
    {
      id: "cursor",
      label: "Cursor",
      steps: [],
      snippets: [
        {
          label: ".cursor/mcp.json",
          text: JSON.stringify({ mcpServers: { memory: { url } } }, null, 2),
        },
      ],
    },
    {
      id: "cli",
      label: "memory CLI",
      steps: [],
      snippets: [{ label: "Command", text: `memory auth login --host ${origin}` }],
    },
  ];
}
