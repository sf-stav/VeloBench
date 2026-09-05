/**
 * Small shared utilities.
 */

/** Generate a reasonably-unique id without relying on `crypto.randomUUID`
 * (which is unavailable on non-secure origins). Mirrors the backend's approach. */
export function uid(short = false): string {
  const time = Date.now().toString(36);
  const rand = Math.floor(Math.random() * 0xffffffff).toString(36);
  const s = `${time}${rand}`;
  return short ? s.slice(0, 8) : s;
}
