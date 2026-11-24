type JsonLdProps = {
  data: Record<string, unknown> | ReadonlyArray<Record<string, unknown>>;
};

const UNSAFE_SCRIPT_CHARS: Record<string, string> = {
  "<": "\\u003c",
  ">": "\\u003e",
  "&": "\\u0026",
  "\u2028": "\\u2028",
  "\u2029": "\\u2029",
};

/**
 * `JSON.stringify` that is safe to inline in a `<script>` element.
 *
 * `<`, `>`, `&`, U+2028 and U+2029 can only appear inside JSON strings, where
 * their `\uXXXX` escapes decode to the same value, so the result is still valid
 * JSON with identical data, but a value such as `"</script>"` can no longer
 * close the surrounding script element.
 */
export function serializeJsonLd(data: JsonLdProps["data"]): string {
  return JSON.stringify(data).replace(
    /[<>&\u2028\u2029]/g,
    (char) => UNSAFE_SCRIPT_CHARS[char],
  );
}

/** Server-safe JSON-LD injector for structured data. */
export function JsonLd({ data }: JsonLdProps) {
  return (
    <script
      type="application/ld+json"
      dangerouslySetInnerHTML={{ __html: serializeJsonLd(data) }}
    />
  );
}
