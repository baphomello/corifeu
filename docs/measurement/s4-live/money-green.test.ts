import { describe, expect, test } from "vitest";
import { formatBRL } from "./money";

describe("formatBRL", () => {
  test("formats zero centavos correctly", () => {
    expect(formatBRL(0)).toBe("R$ 0,00");
  });

  test("formats positive amount with two-digit reais", () => {
    expect(formatBRL(1234)).toBe("R$ 12,34");
  });

  test("formats positive amount with three-digit reais", () => {
    expect(formatBRL(123456)).toBe("R$ 1.234,56");
  });

  test("formats positive amount with four-digit reais", () => {
    expect(formatBRL(1234567)).toBe("R$ 12.345,67");
  });

  test("formats positive amount with five-digit reais", () => {
    expect(formatBRL(12345678)).toBe("R$ 123.456,78");
  });

  test("formats positive amount with six-digit reais", () => {
    expect(formatBRL(123456789)).toBe("R$ 1.234.567,89");
  });

  test("formats negative amount", () => {
    expect(formatBRL(-1234)).toBe("-R$ 12,34");
  });

  test("formats negative amount with large reais", () => {
    expect(formatBRL(-1234567)).toBe("-R$ 12.345,67");
  });

  test("formats amount with centavos less than 10", () => {
    expect(formatBRL(1005)).toBe("R$ 10,05");
  });

  test("formats amount with centavos equal to 0", () => {
    expect(formatBRL(1000)).toBe("R$ 10,00");
  });

  test("formats amount with centavos equal to 99", () => {
    expect(formatBRL(1099)).toBe("R$ 10,99");
  });

  test("formats amount with single digit reais", () => {
    expect(formatBRL(500)).toBe("R$ 5,00");
  });

  test("formats amount with single digit reais and non-zero centavos", () => {
    expect(formatBRL(567)).toBe("R$ 5,67");
  });

  test("throws error for non-number input", () => {
    expect(() => formatBRL("1234" as any)).toThrow();
  });

  test("throws error for NaN input", () => {
    expect(() => formatBRL(NaN)).toThrow();
  });

  test("throws error for Infinity input", () => {
    expect(() => formatBRL(Infinity)).toThrow();
  });

  test("throws error for -Infinity input", () => {
    expect(() => formatBRL(-Infinity)).toThrow();
  });

  test("formats very large amount", () => {
    expect(formatBRL(123456789012)).toBe("R$ 1.234.567.890,12");
  });

  test("formats negative very large amount", () => {
    expect(formatBRL(-123456789012)).toBe("-R$ 1.234.567.890,12");
  });
});
