export function titleForDoc(value: string, fallback = 'Documentation'): string {
  const title = value
    .split('/')
    .filter(Boolean)
    .at(-1)
    ?.replace(/[-_]+/g, ' ')
    .replace(/\b\w/g, (letter) => letter.toUpperCase());

  return title || fallback;
}
