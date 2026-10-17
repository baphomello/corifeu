const ELLIPSIS = "…";

export function excerpt(text: string, limit: number): string {
  if (typeof text !== "string") {
    throw new TypeError("excerpt: text must be a string");
  }
  if (typeof limit !== "number" || Number.isNaN(limit)) {
    throw new TypeError("excerpt: limit must be a number");
  }

  const normalized = text.replace(/\s+/gu, " ").trim();
  const chars = Array.from(normalized);

  if (limit === Infinity || chars.length <= limit) {
    return normalized;
  }

  const max = Math.floor(limit);
  if (max <= 0) {
    return "";
  }
  if (max === 1) {
    return ELLIPSIS;
  }

  const room = max - 1;
  const head = chars.slice(0, room);
  const nextChar = chars[room];

  let cut: string;
  if (nextChar === " ") {
    cut = head.join("");
  } else {
    const lastSpace = head.lastIndexOf(" ");
    cut = lastSpace > 0 ? head.slice(0, lastSpace).join("") : head.join("");
  }

  cut = cut.replace(/[\s.,;:!?…\-–—]+$/u, "");
  if (cut.length === 0) {
    cut = head.join("").trimEnd();
  }

  return cut + ELLIPSIS;
}
