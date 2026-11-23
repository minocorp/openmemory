export function inviteLink(origin: string, token: string): string {
  return `${origin}/register?token=${encodeURIComponent(token)}`;
}
