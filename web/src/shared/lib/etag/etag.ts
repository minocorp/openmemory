export function etagOf(version: number): string {
  return `"${version}"`;
}

export function versionOfEtag(etag: string | null): number | null {
  const match = etag?.match(/^"(\d+)"$/);
  return match?.[1] ? Number(match[1]) : null;
}
