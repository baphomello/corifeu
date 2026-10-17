import { expect, test } from "vitest";
import { slug } from "./slug";

test("lowercases and joins with hyphens", () => {
  expect(slug("Hello World")).toBe("hello-world");
});
test("collapses runs of separators", () => {
  expect(slug("a   b")).toBe("a-b");
});
test("drops punctuation", () => {
  expect(slug("Hello, World!")).toBe("hello-world");
});
test("trims leading and trailing separators", () => {
  expect(slug("  spaced  ")).toBe("spaced");
});
