import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** Match pathlib's sorted rglob for the resource exports, without following symlinks. */
export function walkFiles(directory: string, suffix = ""): string[] {
  if (!existsSync(directory)) return [];
  const files: string[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...walkFiles(path, suffix));
    else if (entry.isFile() && entry.name.endsWith(suffix)) files.push(path);
  }
  return files.sort();
}

/** Small dependency-free equivalent of these tools' argparse option sets. */
export function parseArgs(
  defaults: Record<string, string>,
  description: string,
  flags: readonly string[] = ["check"],
  argv = process.argv.slice(2),
): Record<string, string | boolean> {
  const result: Record<string, string | boolean> = { ...defaults };
  for (const flag of flags) result[flag] = false;
  const fail = (message: string): never => {
    console.error(message);
    process.exit(2);
  };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      console.log(`${description}\n\n${Object.keys(defaults).map(key => `--${key} VALUE (default: ${defaults[key]})`).concat(flags.map(flag => `--${flag}`)).join("\n")}`);
      process.exit(0);
    }
    if (!arg.startsWith("--")) fail(`unrecognized argument: ${arg}`);
    const equal = arg.indexOf("=");
    const name = arg.slice(2, equal < 0 ? undefined : equal);
    if (flags.includes(name)) {
      if (equal >= 0) fail(`argument --${name}: ignored explicit argument`);
      result[name] = true;
    } else if (Object.hasOwn(defaults, name)) {
      if (equal >= 0) result[name] = arg.slice(equal + 1);
      else {
        const value = argv[++i];
        if (value === undefined || value.startsWith("--")) fail(`argument --${name}: expected one argument`);
        result[name] = value;
      }
    } else fail(`unrecognized argument: ${arg}`);
  }
  return result;
}

export function writeOrCheck(output: string, contents: string, check: boolean): void {
  if (check) {
    if (!existsSync(output) || readFileSync(output, "utf8").replace(/\r\n?/g, "\n") !== contents) {
      console.error(`${output} is out of date`);
      process.exit(1);
    }
  } else writeFileSync(output, contents);
}

export function integer(value: string | undefined | null): number {
  if (value === undefined || value === null || !/^[+-]?\d+$/.test(value.trim())) throw new Error(`Invalid integer: ${value}`);
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed)) throw new Error(`Integer outside the exact JavaScript range: ${value}`);
  return parsed;
}
