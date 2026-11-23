let issued = 0;

export function uniqueClientHeaders(): Record<string, string> {
  issued += 1;
  return {
    "X-Forwarded-For": `10.${Math.floor(issued / 250) % 250}.${Math.floor(Math.random() * 250)}.${issued % 250}`,
  };
}
