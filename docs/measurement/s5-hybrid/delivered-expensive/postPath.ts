const ISO_DATE = /^(\d{4})-(\d{2})-(\d{2})(?:T[0-9:.]+(?:Z|[+-]\d{2}:?\d{2})?)?$/;

export function postPath(slug: string, isoDate: string): string {
  if (typeof slug !== "string" || slug.trim() === "" || slug !== slug.trim()) {
    throw new Error(`Invalid slug: ${JSON.stringify(slug)}`);
  }
  if (/[\/\\?#\s]/.test(slug) || slug === "." || slug === "..") {
    throw new Error(`Invalid slug: ${JSON.stringify(slug)}`);
  }
  if (typeof isoDate !== "string") {
    throw new Error(`Invalid ISO date: ${JSON.stringify(isoDate)}`);
  }

  const match = ISO_DATE.exec(isoDate);
  if (!match) {
    throw new Error(`Invalid ISO date: ${JSON.stringify(isoDate)}`);
  }

  const [, yearStr, monthStr, dayStr] = match;
  const year = Number(yearStr);
  const month = Number(monthStr);
  const day = Number(dayStr);

  if (month < 1 || month > 12 || day < 1) {
    throw new Error(`Invalid ISO date: ${JSON.stringify(isoDate)}`);
  }
  const daysInMonth = new Date(Date.UTC(year, month, 0)).getUTCDate();
  if (day > daysInMonth) {
    throw new Error(`Invalid ISO date: ${JSON.stringify(isoDate)}`);
  }

  return `/blog/${yearStr}/${monthStr}/${slug}`;
}
