export interface PythonJsonOptions { indent?: number; sortKeys?: boolean; ensureAscii?: boolean }

/** Python json.dumps formatting keeps source-derived artifacts byte-for-byte stable. */
export function pythonJson(value: unknown, options: PythonJsonOptions = {}): string {
  const { indent = 2, sortKeys = false, ensureAscii = true } = options;
  const quote = (text: string): string => {
    const result = JSON.stringify(text);
    return ensureAscii ? result.replace(/[\u007f-\uffff]/g, char => `\\u${char.charCodeAt(0).toString(16).padStart(4, "0")}`) : result;
  };
  const number = (value: number): string => {
    if (Number.isNaN(value)) return "NaN";
    if (value === Infinity) return "Infinity";
    if (value === -Infinity) return "-Infinity";
    const float = !Number.isInteger(value);
    const magnitude = Math.abs(value);
    if (float && magnitude !== 0 && (magnitude < 0.0001 || magnitude >= 1e16)) {
      return value.toExponential().replace(/e([+-])(\d+)$/, (_, sign, digits) => `e${sign}${digits.padStart(2, "0")}`);
    }
    const result = String(value);
    return float && !/[.e]/.test(result) ? result + ".0" : result;
  };
  const stringify = (item: unknown, depth: number): string => {
    if (item === null) return "null";
    if (typeof item === "string") return quote(item);
    if (typeof item === "boolean") return item ? "true" : "false";
    if (typeof item === "number") return number(item);
    const padding = " ".repeat(depth * indent);
    const innerPadding = " ".repeat((depth + 1) * indent);
    if (Array.isArray(item)) {
      if (!item.length) return "[]";
      return `[\n${item.map(value => innerPadding + stringify(value, depth + 1)).join(",\n")}\n${padding}]`;
    }
    if (item && typeof item === "object") {
      const entries = Object.entries(item);
      // Serialize entries directly: JSON.stringify would reorder integer keys.
      if (sortKeys) entries.sort(([a], [b]) => {
        const left = Array.from(a, char => char.codePointAt(0)!);
        const right = Array.from(b, char => char.codePointAt(0)!);
        for (let index = 0; index < Math.min(left.length, right.length); index++) {
          if (left[index] !== right[index]) return left[index] - right[index];
        }
        return left.length - right.length;
      });
      if (!entries.length) return "{}";
      return `{\n${entries.map(([key, value]) => `${innerPadding}${quote(key)}: ${stringify(value, depth + 1)}`).join(",\n")}\n${padding}}`;
    }
    throw new Error("Value cannot be serialized as JSON");
  };
  return stringify(value, 0);
}
