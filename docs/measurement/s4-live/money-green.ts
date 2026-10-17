/**
 * Format an amount of Brazilian centavos as a BRL string.
 *
 * The argument is a whole number of CENTAVOS, so 1234 means twelve reais and thirty-four centavos and formats as 'R$ 12,34'.
 * The format is: the symbol and a space, then the integer part of the reais with a dot every three digits, then a comma, then exactly two digits of centavos.
 * A negative amount puts the minus sign before the symbol.
 */
export function formatBRL(centavos: number): string {
  // Validate input is a number
  if (typeof centavos !== 'number' || !Number.isFinite(centavos)) {
    throw new Error("Invalid input: must be a finite number");
  }

  // Handle negative numbers by preserving the sign
  const isNegative = centavos < 0;
  const absCentavos = Math.abs(centavos);

  // Calculate reais and centavos
  const reais = Math.floor(absCentavos / 100);
  const centavosPart = absCentavos % 100;

  // Format the reais part with dots every three digits
  const reaisStr = reais.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ".");

  // Format the centavos part with exactly two digits
  const centavosStr = centavosPart.toString().padStart(2, '0');

  // Combine parts
  const formatted = `R$ ${reaisStr},${centavosStr}`;

  // Add minus sign if negative
  if (isNegative) {
    return `-${formatted}`;
  }

  return formatted;
}
