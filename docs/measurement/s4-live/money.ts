/**
 * Format an amount of Brazilian centavos as a BRL string.
 *
 * Formats an integer number of Brazilian centavos as a BRL string:
 * - A dot every three digits of the integer part
 * - A comma before exactly two decimal places
 * - The symbol and a space in front (e.g., 'R$ 1.234,56')
 * - A negative amount puts the minus sign before the symbol (e.g., '-R$ 1.234,56')
 */
export function formatBRL(centavos: number): string {
  // Validate input is an integer
  if (!Number.isInteger(centavos)) {
    throw new Error("centavos must be an integer");
  }

  const negative = centavos < 0;
  const absCentavos = Math.abs(centavos);

  // Split into integer and fractional parts
  const integerPart = Math.floor(absCentavos / 100);
  const fractionalPart = absCentavos % 100;

  // Format integer part with dots every three digits
  const integerStr = integerPart.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ".");

  // Format fractional part with exactly two digits
  const fractionalStr = fractionalPart.toString().padStart(2, "0");

  // Combine parts
  const formatted = `${integerStr},${fractionalStr}`;

  // Add symbol and space
  const result = `R$ ${formatted}`;

  // Add minus sign if negative
  if (negative) {
    return `-R$ ${formatted}`;
  }

  return result;
}
