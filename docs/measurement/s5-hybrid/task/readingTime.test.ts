import { describe, expect, it } from "vitest";
import { readingTime } from "./readingTime";

const words = (n: number) => new Array(n).fill("palavra").join(" ");

describe("readingTime", () => {
  it("counts whole minutes at 200 words a minute", () => {
    expect(readingTime(words(400))).toBe(2);
  });

  it("rounds a part minute up", () => {
    expect(readingTime(words(201))).toBe(2);
  });

  it("never reports less than one minute", () => {
    expect(readingTime("duas palavras")).toBe(1);
  });

  it("treats blank text as one minute", () => {
    expect(readingTime("   ")).toBe(1);
  });
});
