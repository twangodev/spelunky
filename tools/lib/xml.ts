import { readFileSync } from "node:fs";

/** The ElementTree subset used by the untouched GameMaker XML export. */
export class XmlElement {
  children: XmlElement[] = [];
  text: string | null = null;
  tail: string | null = null;

  constructor(public tag: string, public attrib: Record<string, string> = {}) {}

  get attributes(): Record<string, string> { return this.attrib; }
  get(name: string, fallback: string | null = null): string | null {
    return Object.hasOwn(this.attrib, name) ? this.attrib[name] : fallback;
  }
  *iter(tag?: string): Generator<XmlElement> {
    if (tag === undefined || tag === "*" || tag === this.tag) yield this;
    for (const child of this.children) yield* child.iter(tag);
  }
  findAll(path: string): XmlElement[] {
    const descendant = path.startsWith(".//") || path.startsWith("//");
    const parts = path.replace(/^\.\/\//, "").replace(/^\/\//, "").replace(/^\.\//, "").split("/");
    if (path === ".") return [this];
    let candidates = descendant ? [...this.iter()].slice(1) : this.children;
    let result: XmlElement[] = [];
    for (let index = 0; index < parts.length; index++) {
      const part = parts[index];
      const match = /^([^\[]+)(?:\[@([^=\]]+)=(['"])(.*?)\3\])?$/.exec(part);
      if (!match) throw new Error(`Unsupported XML path: ${path}`);
      result = candidates.filter(node => (match[1] === "*" || node.tag === match[1]) && (!match[2] || node.get(match[2]) === match[4]));
      if (index + 1 < parts.length) candidates = result.flatMap(node => node.children);
    }
    return result;
  }
  find(path: string): XmlElement | null { return this.findAll(path)[0] ?? null; }
  findText(path: string, fallback: string | null = null): string | null {
    const node = this.find(path);
    return node ? node.text ?? "" : fallback;
  }
}

function decodeEntities(value: string): string {
  return value.replace(/&([^;\s<&]+);|&/g, (_, entity: string | undefined) => {
    if (entity === undefined) throw new Error("Unescaped ampersand in XML");
    const named: Record<string, string> = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'" };
    if (Object.hasOwn(named, entity)) return named[entity];
    if (/^#(?:[0-9]+|x[0-9a-fA-F]+)$/.test(entity)) {
      const number = entity.startsWith("#x") ? parseInt(entity.slice(2), 16) : parseInt(entity.slice(1), 10);
      if (number === 9 || number === 10 || number === 13 || number >= 0x20 && number <= 0xd7ff || number >= 0xe000 && number <= 0xfffd || number >= 0x10000 && number <= 0x10ffff) return String.fromCodePoint(number);
    }
    throw new Error(`Invalid or unsupported XML entity: &${entity};`);
  });
}

export function parseXml(raw: string | Uint8Array): XmlElement {
  const source = (typeof raw === "string" ? raw : new TextDecoder("utf-8", { fatal: true }).decode(raw)).replace(/^\uFEFF/, "").replace(/\r\n?/g, "\n");
  const stack: XmlElement[] = [];
  let root: XmlElement | null = null;
  let cursor = 0;
  const append = (value: string) => {
    const parent = stack.at(-1);
    if (!parent) {
      if (value.trim()) throw new Error("Text outside XML root");
      return;
    }
    const previous = parent.children.at(-1);
    if (previous) previous.tail = (previous.tail ?? "") + value;
    else parent.text = (parent.text ?? "") + value;
  };
  while (cursor < source.length) {
    const open = source.indexOf("<", cursor);
    if (open < 0) { append(decodeEntities(source.slice(cursor))); break; }
    if (open > cursor) append(decodeEntities(source.slice(cursor, open)));
    if (source.startsWith("<!--", open)) {
      const end = source.indexOf("-->", open + 4);
      if (end < 0) throw new Error("Unclosed XML comment");
      cursor = end + 3; continue;
    }
    if (source.startsWith("<![CDATA[", open)) {
      const end = source.indexOf("]]>", open + 9);
      if (end < 0) throw new Error("Unclosed XML CDATA");
      append(source.slice(open + 9, end)); cursor = end + 3; continue;
    }
    if (source.startsWith("<?", open)) {
      const end = source.indexOf("?>", open + 2);
      if (end < 0) throw new Error("Unclosed XML processing instruction");
      cursor = end + 2; continue;
    }
    if (source.startsWith("<!", open)) throw new Error("XML document type declarations are unsupported");
    let end = open + 1;
    let quote = "";
    for (; end < source.length; end++) {
      const char = source[end];
      if (quote) { if (char === quote) quote = ""; }
      else if (char === '"' || char === "'") quote = char;
      else if (char === ">") break;
    }
    if (end === source.length) throw new Error("Unclosed XML tag");
    const content = source.slice(open + 1, end);
    cursor = end + 1;
    if (content.startsWith("/")) {
      const node = stack.pop();
      if (!node || node.tag !== content.slice(1).trim()) throw new Error(`Mismatched XML closing tag: ${content}`);
      continue;
    }
    const empty = content.endsWith("/");
    const body = empty ? content.slice(0, -1) : content;
    const name = /^([^\s/>]+)/.exec(body);
    if (!name) throw new Error("Missing XML element name");
    const attributes: Record<string, string> = {};
    let offset = name[0].length;
    while (offset < body.length) {
      const rest = body.slice(offset);
      if (!rest.trim()) break;
      const attribute = /^\s+([^\s=/>]+)\s*=\s*(['"])(.*?)\2/s.exec(rest);
      if (!attribute) throw new Error(`Invalid XML attribute in ${body}`);
      if (Object.hasOwn(attributes, attribute[1])) throw new Error(`Duplicate XML attribute: ${attribute[1]}`);
      if (attribute[3].includes("<")) throw new Error("Unescaped less-than sign in XML attribute");
      // XML normalizes literal whitespace before resolving character references.
      attributes[attribute[1]] = decodeEntities(attribute[3].replace(/[\n\t]/g, " "));
      offset += attribute[0].length;
    }
    const node = new XmlElement(name[0], attributes);
    const parent = stack.at(-1);
    if (parent) parent.children.push(node);
    else {
      if (root) throw new Error("Multiple XML roots");
      root = node;
    }
    if (!empty) stack.push(node);
  }
  if (!root || stack.length) throw new Error("Missing or unclosed XML root");
  return root;
}

export function readXml(path: string): XmlElement { return parseXml(readFileSync(path)); }

export function serializeXml(element: XmlElement): string {
  const escapeText = (value: string) => value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  const escapeAttribute = (value: string) => escapeText(value).replace(/"/g, "&quot;").replace(/\t/g, "&#09;").replace(/\n/g, "&#10;").replace(/\r/g, "&#13;");
  const attributes = Object.entries(element.attrib).map(([key, value]) => ` ${key}="${escapeAttribute(value)}"`).join("");
  const body = escapeText(element.text ?? "") + element.children.map(serializeXml).join("");
  return (body ? `<${element.tag}${attributes}>${body}</${element.tag}>` : `<${element.tag}${attributes} />`) + escapeText(element.tail ?? "");
}
