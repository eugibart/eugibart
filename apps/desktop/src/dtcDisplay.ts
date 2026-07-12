// The DTC library honestly marks descriptions that are educated guesses
// with a "(placeholder)" suffix in the data files. In the UI that suffix
// reads like a rendering bug — show it as the same amber "unverified" badge
// used everywhere else instead. Data files stay untouched.
export function splitPlaceholder(description: string): {
  text: string;
  placeholder: boolean;
} {
  const m = description.match(/^(.*\S)\s*\(placeholder\)\s*$/i);
  if (m) return { text: m[1], placeholder: true };
  return { text: description, placeholder: false };
}
