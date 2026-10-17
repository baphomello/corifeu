// Ten traps. The model is shown none of this.
//
// A probe, not a measurement round: one generation, scored, timed. It exists to
// rank models and runtime flags against each other, which is a different
// question from how a doer behaves over many steps.
//
// CPF was chosen because a naive implementation passes six or seven of these
// and fails the rest, which is what makes it discriminate. The two that catch
// almost everyone are the repeated-digit rule — 111.111.111-11 satisfies the
// checksum and is still invalid — and the remainder rule, where a remainder
// below two means the digit is zero rather than eleven minus the remainder.

import { describe, expect, test } from "vitest";
import { isValidCPF } from "./answer";

describe("cpf", () => {
  // 1. The ordinary case, formatted as a Brazilian would write it.
  test("accepts a valid formatted CPF", () => {
    expect(isValidCPF("529.982.247-25")).toBe(true);
  });

  // 2. The same number with nothing between the digits.
  test("accepts a valid unformatted CPF", () => {
    expect(isValidCPF("52998224725")).toBe(true);
  });

  // 3. One digit off in the body must change the check digits.
  test("rejects a CPF whose check digits do not match", () => {
    expect(isValidCPF("529.982.247-26")).toBe(false);
    expect(isValidCPF("111.444.777-36")).toBe(false);
  });

  // 4. The trap. Every repeated-digit CPF satisfies the checksum arithmetic and
  //    every one of them is invalid.
  test("rejects repeated digits even though they satisfy the checksum", () => {
    for (const d of "0123456789") {
      expect(isValidCPF(d.repeat(11))).toBe(false);
    }
  });

  // 5. The other trap. When the remainder is below two the digit is zero, not
  //    eleven minus the remainder. Both of these were verified against the
  //    algorithm rather than invented: the first has a remainder of 1 on the
  //    first digit, and the second has a remainder below two on both.
  test("accepts a CPF whose check digit comes from the remainder rule", () => {
    expect(isValidCPF("526.018.159-06")).toBe(true);
    expect(isValidCPF("743.668.759-00")).toBe(true);
  });

  // 6. Ten or twelve digits is not a CPF.
  test("rejects the wrong length", () => {
    expect(isValidCPF("5299822472")).toBe(false);
    expect(isValidCPF("529982247255")).toBe(false);
  });

  // 7. Letters are not digits, however many there are.
  test("rejects anything that is not a digit", () => {
    expect(isValidCPF("529.982.247-2a")).toBe(false);
    expect(isValidCPF("abcdefghijk")).toBe(false);
  });

  // 8. Empty and whitespace are not a CPF either.
  test("rejects empty and blank input", () => {
    expect(isValidCPF("")).toBe(false);
    expect(isValidCPF("   ")).toBe(false);
  });

  // 9. Separators in the wrong places are still separators; the digits decide.
  test("ignores where the separators fall", () => {
    expect(isValidCPF("52998-224.725")).toBe(true);
  });

  // 10. The second check digit is computed over the first, so a correct first
  //     digit with a wrong second must fail.
  test("computes the second check digit over the first", () => {
    expect(isValidCPF("529.982.247-20")).toBe(false);
    expect(isValidCPF("529.982.247-05")).toBe(false);
  });
});
