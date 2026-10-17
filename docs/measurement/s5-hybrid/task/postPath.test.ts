import { describe, expect, it } from "vitest";
import { postPath } from "./postPath";

describe("postPath", () => {
  it("puts the year and the month before the slug", () => {
    expect(postPath("meu-post", "2026-10-17")).toBe("/blog/2026/10/meu-post");
  });

  it("keeps a one-digit month padded", () => {
    expect(postPath("outro", "2026-03-04")).toBe("/blog/2026/03/outro");
  });

  it("refuses a date it cannot read", () => {
    expect(() => postPath("x", "17/10/2026")).toThrow();
  });
});
