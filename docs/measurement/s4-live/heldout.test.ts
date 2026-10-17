// Held out. The doer never sees this file; it is run after the slice closes,
// to say whether the tests the doer wrote for itself meant anything.
//
// Ten cases, and the traps are the ones a naive implementation gets wrong:
// where the thousands separator goes once there are two of them, that the
// minus sign belongs outside the currency symbol, and that five centavos is
// "0,05" rather than "0,5".
import { describe, expect, test } from "vitest";
import { formatBRL } from "./money";

describe("formatBRL, held out", () => {
  test("zero", () => expect(formatBRL(0)).toBe("R$ 0,00"));
  test("a single centavo pads", () => expect(formatBRL(5)).toBe("R$ 0,05"));
  test("fifty centavos does not pad", () => expect(formatBRL(50)).toBe("R$ 0,50"));
  test("one real", () => expect(formatBRL(100)).toBe("R$ 1,00"));
  test("under a thousand has no separator", () => expect(formatBRL(99999)).toBe("R$ 999,99"));
  test("one separator", () => expect(formatBRL(123456)).toBe("R$ 1.234,56"));
  test("two separators", () => expect(formatBRL(100000000)).toBe("R$ 1.000.000,00"));
  test("many separators", () => expect(formatBRL(999999999999)).toBe("R$ 9.999.999.999,99"));
  test("negative keeps the sign outside", () => expect(formatBRL(-123456)).toBe("-R$ 1.234,56"));
  test("negative centavos", () => expect(formatBRL(-5)).toBe("-R$ 0,05"));
});
