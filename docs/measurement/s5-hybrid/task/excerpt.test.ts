import { describe, expect, it } from "vitest";
import { excerpt } from "./excerpt";

describe("excerpt", () => {
  it("leaves text shorter than the limit alone", () => {
    expect(excerpt("curto", 20)).toBe("curto");
  });

  it("cuts on a word boundary and marks the cut", () => {
    expect(excerpt("um texto bem mais longo do que caberia", 14)).toBe("um texto bem…");
  });

  it("counts the ellipsis inside the limit", () => {
    expect(excerpt("alfa beta gama", 10)).toBe("alfa beta…");
  });
});
