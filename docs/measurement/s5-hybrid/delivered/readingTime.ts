export function readingTime(text: string): number {
  const wordCount = text.trim() === "" ? 0 : text.trim().split(/\s+/).length;
  const minutes = Math.ceil(wordCount / 200);
  return Math.max(1, minutes);
}
