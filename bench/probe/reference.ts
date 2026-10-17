// The answer, for checking that the spec measures the work and not itself.
// Never shown to a model.
export function isValidCPF(cpf: string): boolean {
  const ds = [...cpf].filter((c) => c >= "0" && c <= "9").map(Number);
  if (ds.length !== 11) return false;
  if (/[^0-9.\-\s]/.test(cpf)) return false;
  if (ds.every((d) => d === ds[0])) return false;

  const digit = (upto: number): number => {
    const w = upto + 1;
    const sum = ds.slice(0, upto).reduce((acc, d, i) => acc + d * (w - i), 0);
    const r = sum % 11;
    return r < 2 ? 0 : 11 - r;
  };
  return ds[9] === digit(9) && ds[10] === digit(10);
}
