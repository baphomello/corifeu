```typescript
export function isValidCPF(cpf: string): boolean {
  // Remove all non-digit characters
  const digits = cpf.replace(/[^0-9]/g, '');

  // Must be exactly 11 digits
  if (digits.length !== 11) return false;

  // All digits must be the same (e.g., "11111111111")
  if (/^(\d)\1+$/.test(digits)) return false;

  // Calculate the first check digit
  let sum = 0;
  for (let i = 0; i < 9; i++) {
    sum += parseInt(digits[i], 10) * (10 - i);
  }
  const firstDigit = (sum * 10) % 11;
  const firstDigitResult = firstDigit === 10 ? 1 : firstDigit;
  if (parseInt(digits[9], 10) !== firstDigitResult) return false;

  // Calculate the second check digit
  sum = 0;
  for (let i = 0; i < 10; i++) {
    sum += parseInt(digits[i], 10) * (11 - i);
  }
  const secondDigit = (sum * 10) % 11;
  const secondDigitResult = secondDigit === 10 ? 1 : secondDigit;
  if (parseInt(digits[10], 10) !== secondDigitResult) return false;

  return true;
}
```