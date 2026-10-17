import { describe, expect, test } from "vitest";
import { formatBRL } from "./money";

describe("formatBRL", () => {
  test("formats zero centavos correctly", () => {
    expect(formatBRL(0)).toBe("R$ 0,00");
  });

  test("formats positive amount with two digits", () => {
    expect(formatBRL(100)).toBe("R$ 1,00");
  });

  test("formats positive amount with three digits", () => {
    expect(formatBRL(1234)).toBe("R$ 1.234,00");
  });

  test("formats positive amount with four digits", () => {
    expect(formatBRL(12345)).toBe("R$ 12.345,00");
  });

  test("formats positive amount with five digits", () => {
    expect(formatBRL(123456)).toBe("R$ 1.234,56");
  });

  test("formats positive amount with six digits", () => {
    expect(formatBRL(1234567)).toBe("R$ 12.345,67");
  });

  test("formats negative amount", () => {
    expect(formatBRL(-123456)).toBe("-R$ 1.234,56");
  });

  test("formats negative amount with zero", () => {
    expect(formatBRL(-100)).toBe("-R$ 1,00");
  });

  test("throws error for non-integer input", () => {
    expect(() => formatBRL(1.5)).toThrow("centavos must be an integer");
  });

  test("throws error for negative non-integer input", () => {
    expect(() => formatBRL(-1.5)).toThrow("centavos must be an integer");
  });

  test("formats large positive amount", () => {
    expect(formatBRL(1234567890)).toBe("R$ 12.345.678,90");
  });

  test("formats large negative amount", () => {
    expect(formatBRL(-1234567890)).toBe("-R$ 12.345.678,90");
  });
});
