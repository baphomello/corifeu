export function excerpt(text: string, limit: number): string {
  if (text.length <= limit) {
    return text;
  }

  const words = text.split(" ");
  let result = "";
  let currentLength = 0;

  for (let i = 0; i < words.length; i++) {
    const word = words[i];
    const wordLength = word.length;

    if (currentLength + wordLength + 1 > limit) {
      // Need to cut, add ellipsis
      result += "…";
      break;
    }

    if (i > 0) {
      result += " ";
      currentLength++;
    }
    result += word;
    currentLength += wordLength;
  }

  return result;
}
