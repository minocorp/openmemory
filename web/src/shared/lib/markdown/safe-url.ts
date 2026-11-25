const allowedProtocols = new Set(["http:", "https:", "mailto:"]);
const emailPattern = /^[^\s@/:]+@[^\s@/:]+\.[^\s@/:]+$/;

export function safeHref(raw: string): string | null {
  const trimmed = raw.trim();
  if (emailPattern.test(trimmed)) {
    return `mailto:${trimmed}`;
  }
  const candidate = /^www\./i.test(trimmed) ? `https://${trimmed}` : trimmed;
  if (!URL.canParse(candidate)) {
    return null;
  }
  const url = new URL(candidate);
  return allowedProtocols.has(url.protocol) ? url.href : null;
}
