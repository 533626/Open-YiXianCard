// Mirrors the original client's plain-text description parsing
// (ConfigExtension.ParsePlainDescription / TalentConfig.ParseDescription):
// substitute {field} / {otherParams[N]} placeholders, then drop the
// [keyword] bracket markup and <color>/<size>/<i> rich-text tags the
// client renders visually (a plain title="" tooltip can't render either).
export function formatOriginalDetail(
  template: string,
  otherParams: readonly (number | string | undefined)[] = [],
  fields: Readonly<Record<string, number | string | undefined>> = {},
): string {
  if (!template) return "";
  let text = template.split("\\n").join("\n");
  for (const [key, value] of Object.entries(fields)) {
    if (value === undefined) continue;
    text = text.split(`{${key}}`).join(String(value));
  }
  otherParams.forEach((value, index) => {
    if (value === undefined) return;
    text = text.split(`{otherParams[${index}]}`).join(String(value));
  });
  // Fields the extraction pipeline doesn't carry yet (e.g. randomDef) stay
  // as unresolved {token} placeholders — drop them rather than leak raw syntax.
  return text
    .replace(/\{[^}]*\}/g, "")
    .replace(/<[^>]+>/g, "")
    .replace(/[[\]]/g, "");
}
