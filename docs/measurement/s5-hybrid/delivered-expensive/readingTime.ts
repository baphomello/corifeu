const WORDS_PER_MINUTE = 200;

export function readingTime(text: string): number {
  if (typeof text !== "string") return 1;
  const words = text.trim().split(/\s+/u).filter((w) => w.length > 0).length;
  return Math.max(1, Math.ceil(words / WORDS_PER_MINUTE));
}
