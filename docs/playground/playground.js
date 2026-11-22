const PRESETS = {
  iso20022: {
    xsd: `<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="urn:example:payments"
           xmlns:tns="urn:example:payments"
           elementFormDefault="qualified">
  <xs:simpleType name="CurrencyCode">
    <xs:restriction base="xs:string">
      <xs:enumeration value="USD"/>
      <xs:enumeration value="EUR"/>
      <xs:enumeration value="GBP"/>
    </xs:restriction>
  </xs:simpleType>

  <xs:complexType name="Amount">
    <xs:simpleContent>
      <xs:extension base="xs:decimal">
        <xs:attribute name="currency" type="tns:CurrencyCode" use="required"/>
      </xs:extension>
    </xs:simpleContent>
  </xs:complexType>

  <xs:complexType name="PaymentTransfer">
    <xs:sequence>
      <xs:element name="TransferId" type="xs:string"/>
      <xs:element name="Amount" type="tns:Amount"/>
      <xs:choice>
        <xs:element name="Iban" type="xs:string"/>
        <xs:element name="RoutingNumber" type="xs:string"/>
      </xs:choice>
    </xs:sequence>
  </xs:complexType>

  <xs:element name="PaymentTransfer" type="tns:PaymentTransfer"/>
</xs:schema>`,
    xml: `<?xml version="1.0" encoding="UTF-8"?>
<PaymentTransfer xmlns="urn:example:payments">
  <TransferId>TX-2026-00419</TransferId>
  <Amount currency="USD">14250.75</Amount>
  <Iban>DE89370400440532013000</Iban>
</PaymentTransfer>`,
  },
  defense: {
    xsd: `<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="urn:example:telemetry"
           xmlns:tns="urn:example:telemetry"
           elementFormDefault="qualified">
  <xs:complexType name="TelemetryHeader">
    <xs:sequence>
      <xs:element name="PlatformId" type="xs:string"/>
      <xs:element name="Seq" type="xs:unsignedLong"/>
    </xs:sequence>
  </xs:complexType>

  <xs:complexType name="TrackNode">
    <xs:complexContent>
      <xs:extension base="tns:TelemetryHeader">
        <xs:sequence>
          <xs:choice>
            <xs:element name="BearingDeg" type="xs:double"/>
            <xs:element name="LostSignal" type="xs:boolean"/>
          </xs:choice>
          <xs:element name="Child" type="tns:TrackNode" minOccurs="0" maxOccurs="unbounded"/>
        </xs:sequence>
      </xs:extension>
    </xs:complexContent>
  </xs:complexType>

  <xs:element name="TrackNode" type="tns:TrackNode"/>
</xs:schema>`,
    xml: `<TrackNode xmlns="urn:example:telemetry">
  <PlatformId>UAV-ALPHA</PlatformId>
  <Seq>9012</Seq>
  <BearingDeg>137.5</BearingDeg>
  <Child>
    <PlatformId>RELAY-02</PlatformId>
    <Seq>9013</Seq>
    <LostSignal>false</LostSignal>
  </Child>
</TrackNode>`,
  },
  ubl: {
    xsd: `<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="urn:example:invoice"
           xmlns:tns="urn:example:invoice"
           elementFormDefault="qualified">
  <xs:complexType name="LineItem">
    <xs:sequence>
      <xs:element name="Sku" type="xs:string"/>
      <xs:element name="Quantity" type="xs:int"/>
      <xs:element name="UnitPrice" type="xs:decimal"/>
    </xs:sequence>
    <xs:attribute name="taxExempt" type="xs:boolean" default="false"/>
  </xs:complexType>

  <xs:complexType name="Invoice">
    <xs:sequence>
      <xs:element name="InvoiceId" type="xs:string"/>
      <xs:element name="IssueDate" type="xs:date"/>
      <xs:element name="Line" type="tns:LineItem" maxOccurs="unbounded"/>
    </xs:sequence>
  </xs:complexType>

  <xs:element name="Invoice" type="tns:Invoice"/>
</xs:schema>`,
    xml: `<Invoice xmlns="urn:example:invoice">
  <InvoiceId>INV-2026-108</InvoiceId>
  <IssueDate>2026-11-22</IssueDate>
  <Line taxExempt="false">
    <Sku>POLY-CORE-01</Sku>
    <Quantity>4</Quantity>
    <UnitPrice>250.00</UnitPrice>
  </Line>
</Invoice>`,
  },
  upa: {
    xsd: `<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="AmbiguousOrder">
    <xs:complexType>
      <xs:sequence>
        <!-- W3C XSD 1.0 Unique Particle Attribution violation:
             an <Item> element cannot be unambiguously attributed
             to the optional first particle vs required second particle. -->
        <xs:element name="Item" type="xs:string" minOccurs="0"/>
        <xs:element name="Item" type="xs:string"/>
      </xs:sequence>
    </xs:complexType>
  </xs:element>
</xs:schema>`,
    xml: `<AmbiguousOrder><Item>test</Item></AmbiguousOrder>`,
  },
};

const VARIANTS = {
  rust: [
    { value: "zero-copy", label: "Zero-copy (Cow<'a, str>)" },
    { value: "owned", label: "Owned (String)" },
    { value: "phf", label: "PHF tag dispatch" },
  ],
  python: [
    { value: "dataclass", label: "@dataclass(slots=True)" },
    { value: "pydantic", label: "Pydantic v2 BaseModel" },
  ],
  cpp: [
    { value: "header", label: "Header-only (.hpp, C++20)" },
    { value: "glaze", label: "Glaze JSON serde (glz::meta)" },
    { value: "module", label: "C++20 Module (.cppm)" },
  ],
  java: [
    { value: "record", label: "Java 22+ Records (zero-dep)" },
    { value: "jackson3", label: "Jackson 3 + Bean Validation" },
    { value: "pojo", label: "POJO + Builders (Jackson 3)" },
  ],
  go: [
    { value: "standard", label: "Standard (encoding/xml + json)" },
    { value: "sonic", label: "ByteDance Sonic JSON" },
    { value: "easyjson", label: "EasyJSON" },
  ],
  typescript: [
    { value: "interface", label: "TypeScript 5+ Interfaces" },
    { value: "zod", label: "Zod runtime schemas" },
    { value: "valibot", label: "Valibot runtime schemas" },
    { value: "typebox", label: "TypeBox JSON Schema" },
  ],
  csharp: [
    { value: "record", label: "C# 12 Records" },
    { value: "source-gen", label: "System.Text.Json Source-Gen" },
    { value: "class", label: "Mutable Classes" },
  ],
};

const MAX_XSD_BYTES = 256 * 1024;
const MAX_TRANSCODE_BYTES = 512 * 1024;
const MAX_SHARE_HASH_CHARS = 16_384;
const MAX_DECODE_HASH_CHARS = 32_768;
const MAX_HIGHLIGHT_CHARS = 120_000;
const MAX_HIGHLIGHT_TOKENS = 6_000;

const UPA_AMBIGUOUS_RE =
  /([ \t]*)<xs:element\s+name="Item"\s+type="xs:string"\s+minOccurs="0"\s*\/>\r?\n[ \t]*<xs:element\s+name="Item"\s+type="xs:string"\s*\/>/;

const HIGHLIGHT_BUCKETS = [
  "keyword",
  "type",
  "string",
  "comment",
  "number",
  "attr",
  "fn",
];

const LANG_SYNTAX = {
  rust: {
    keywords: new Set([
      "pub", "struct", "enum", "impl", "fn", "let", "mut", "const", "static",
      "type", "use", "mod", "match", "if", "else", "for", "in", "while", "loop",
      "return", "self", "Self", "where", "as", "crate", "super", "ref", "true",
      "false", "async", "await", "dyn",
    ]),
    builtins: new Set([
      "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64",
      "i128", "isize", "f32", "f64", "bool", "str", "char", "String", "Vec",
      "Option", "Result", "Box", "Cow",
    ]),
    rustAttrs: true,
    rustRawStrings: true,
  },
  python: {
    keywords: new Set([
      "from", "import", "as", "class", "def", "return", "if", "elif", "else",
      "for", "in", "while", "with", "try", "except", "finally", "raise", "pass",
      "None", "True", "False", "and", "or", "not", "is", "lambda", "match",
      "case", "type",
    ]),
    builtins: new Set([
      "str", "int", "float", "bool", "bytes", "list", "dict", "set", "tuple",
      "Optional", "Union", "Annotated", "Literal", "Any", "Self", "Decimal",
    ]),
    hashComment: true,
    cComments: false,
    atAnnotation: true,
    tripleQuotes: true,
    singleQuoteString: true,
  },
  cpp: {
    keywords: new Set([
      "export", "module", "import", "namespace", "struct", "class", "enum",
      "using", "template", "typename", "concept", "requires", "const",
      "constexpr", "consteval", "inline", "static", "virtual", "override",
      "final", "public", "private", "protected", "return", "if", "else",
      "switch", "case", "default", "for", "while", "auto", "void", "nullptr",
      "true", "false", "noexcept", "operator",
    ]),
    builtins: new Set([
      "bool", "char", "int", "short", "long", "unsigned", "signed", "float",
      "double", "size_t", "int8_t", "int16_t", "int32_t", "int64_t",
      "uint8_t", "uint16_t", "uint32_t", "uint64_t", "string", "string_view",
      "vector", "optional", "variant", "unique_ptr",
    ]),
    hashDirective: true,
  },
  java: {
    keywords: new Set([
      "package", "import", "public", "private", "protected", "static", "final",
      "abstract", "sealed", "permits", "record", "class", "interface", "enum",
      "extends", "implements", "return", "if", "else", "switch", "case",
      "default", "for", "while", "new", "null", "true", "false", "var", "void",
      "throws", "throw", "this", "super",
    ]),
    builtins: new Set([
      "boolean", "byte", "short", "int", "long", "float", "double", "char",
      "String", "BigDecimal", "BigInteger", "List", "Optional", "Objects",
    ]),
    atAnnotation: true,
    tripleQuotes: true,
  },
  go: {
    keywords: new Set([
      "package", "import", "type", "struct", "interface", "func", "var",
      "const", "return", "if", "else", "switch", "case", "default", "for",
      "range", "nil", "true", "false", "map", "chan", "defer",
    ]),
    builtins: new Set([
      "string", "bool", "byte", "rune", "error", "any", "int", "int8", "int16",
      "int32", "int64", "uint", "uint8", "uint16", "uint32", "uint64",
      "float32", "float64",
    ]),
    backtick: "attr",
  },
  typescript: {
    keywords: new Set([
      "import", "export", "from", "as", "type", "interface", "enum", "class",
      "extends", "implements", "const", "let", "function", "return", "if",
      "else", "switch", "case", "default", "readonly", "typeof", "keyof",
      "null", "undefined", "true", "false", "new", "void", "never", "unknown",
      "any",
    ]),
    builtins: new Set([
      "string", "number", "boolean", "bigint", "symbol", "object", "Record",
      "Array",
    ]),
    atAnnotation: true,
    backtick: "string",
    singleQuoteString: true,
  },
  csharp: {
    keywords: new Set([
      "namespace", "using", "public", "private", "protected", "internal",
      "static", "sealed", "partial", "abstract", "record", "class", "struct",
      "enum", "interface", "required", "init", "get", "set", "return", "if",
      "else", "switch", "case", "default", "foreach", "for", "while", "var",
      "null", "true", "false", "new", "void", "nameof", "typeof",
    ]),
    builtins: new Set([
      "string", "bool", "byte", "sbyte", "short", "ushort", "int", "uint",
      "long", "ulong", "float", "double", "decimal", "char", "object", "List",
      "IReadOnlyList",
    ]),
    csharpAttrs: true,
    csharpVerbatim: true,
  },
};

const presetSelect = document.getElementById("preset-select");
const shareBtn = document.getElementById("share-btn");
const resetBtn = document.getElementById("reset-btn");
const xsdInput = document.getElementById("xsd-input");
const xsdHighlight = document.getElementById("xsd-highlight");
const irStats = document.getElementById("ir-stats");
const statusLive = document.getElementById("status-live");
const errorBanner = document.getElementById("error-banner");
const errorText = document.getElementById("error-text");
const fixUpaBtn = document.getElementById("fix-upa-btn");

const tablist = document.querySelector(".tablist");
const tabs = Array.from(tablist.querySelectorAll('[role="tab"]'));
const codePanel = document.getElementById("panel-code");
const transcodePanel = document.getElementById("panel-transcode");
const variantSelect = document.getElementById("variant-select");
const fileTabs = document.getElementById("file-tabs");
const compileTiming = document.getElementById("compile-timing");
const copyCodeBtn = document.getElementById("copy-code-btn");
const codeOutput = document.getElementById("code-output");
const generatedCode = document.getElementById("generated-code");

const xmlInput = document.getElementById("xml-input");
const xmlHighlight = document.getElementById("xml-highlight");
const jsonInput = document.getElementById("json-input");
const jsonHighlight = document.getElementById("json-highlight");
const xmlToJsonBtn = document.getElementById("xml-to-json-btn");
const jsonToXmlBtn = document.getElementById("json-to-xml-btn");

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });
const buttonTimers = new WeakMap();
const highlightStore = new Map();

let wasm = null;
let wasmError = null;
let activeTab = "rust";
let activeFiles = [];
let activeFileIndex = 0;
let activeFileName = "";
let activeVariants = {};
let compileTimer = null;
let lastStatusMessage = "";
let cachedStatsXsd = null;
let cachedStatsSummary = "";

function announce(message) {
  let next = message;
  if (next && next === lastStatusMessage) {
    next += "\u00A0";
  }
  lastStatusMessage = message;
  statusLive.textContent = next;
}

function reportCopyStatus(button, ok, message) {
  const tip = button.nextElementSibling;
  clearTimeout(buttonTimers.get(button));
  button.removeAttribute("data-copied");
  button.removeAttribute("data-failed");
  let nextMsg = message;
  if (nextMsg === lastStatusMessage) {
    nextMsg += "\u00A0";
  }
  lastStatusMessage = message;
  if (tip) tip.textContent = "";
  void button.offsetWidth;
  button.setAttribute(ok ? "data-copied" : "data-failed", "");
  requestAnimationFrame(() => {
    if (tip) tip.textContent = nextMsg;
  });
  buttonTimers.set(
    button,
    setTimeout(() => {
      button.removeAttribute("data-copied");
      button.removeAttribute("data-failed");
      if (tip) tip.textContent = "";
    }, 1800)
  );
}

async function writeToClipboard(text) {
  await navigator.clipboard.writeText(text);
}

function showError(message, { showUpaFix = false } = {}) {
  if (!message) {
    errorText.textContent = "";
    fixUpaBtn.hidden = true;
    errorBanner.hidden = true;
    return;
  }
  errorText.textContent = message;
  fixUpaBtn.hidden = !showUpaFix;
  errorBanner.hidden = false;
}

function utf8BytesExceeds(text, limit) {
  // Every valid JS UTF-16 code unit occupies at least 1 UTF-8 byte.
  if (text.length > limit) return text.length;
  const bytes = encoder.encode(text).byteLength;
  return bytes > limit ? bytes : 0;
}

function clearUrlHash() {
  if (location.hash) {
    history.replaceState(null, "", `${location.pathname}${location.search}`);
  }
}

function encodeShareState(state) {
  const bytes = encoder.encode(JSON.stringify(state));
  const chunks = [];
  for (let i = 0; i < bytes.length; i += 0x8000) {
    chunks.push(String.fromCharCode(...bytes.subarray(i, i + 0x8000)));
  }
  const token = btoa(chunks.join(""))
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/g, "");
  return token.length > MAX_SHARE_HASH_CHARS ? null : token;
}

function decodeShareState(hash) {
  if (!hash || !hash.startsWith("#s=") || hash.length > MAX_DECODE_HASH_CHARS) {
    return null;
  }
  try {
    let b64 = hash.slice(3).replace(/-/g, "+").replace(/_/g, "/");
    while (b64.length % 4) b64 += "=";
    const bin = atob(b64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) {
      bytes[i] = bin.charCodeAt(i);
    }
    return JSON.parse(decoder.decode(bytes));
  } catch {
    return null;
  }
}

function isIdentStart(ch) {
  const c = ch.charCodeAt(0);
  return (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c === 95;
}

function isIdentChar(ch) {
  const c = ch.charCodeAt(0);
  return (
    (c >= 65 && c <= 90) ||
    (c >= 97 && c <= 122) ||
    (c >= 48 && c <= 57) ||
    c === 95
  );
}

function isDigit(ch) {
  const c = ch.charCodeAt(0);
  return c >= 48 && c <= 57;
}

function isPascalCaseType(word) {
  const c0 = word.charCodeAt(0);
  if (c0 < 65 || c0 > 90 || word.length < 2) return false;
  for (let i = 1; i < word.length; i++) {
    const c = word.charCodeAt(i);
    if (c >= 97 && c <= 122) return true;
  }
  return false;
}

function scanQuoted(code, start, quote, allowMultiline) {
  let i = start + 1;
  while (i < code.length) {
    const ch = code[i];
    if (ch === "\\") {
      i += 2;
      continue;
    }
    if (ch === quote) return i + 1;
    if (!allowMultiline && ch === "\n") return i;
    i++;
  }
  return code.length;
}

function isXmlNameChar(ch) {
  const c = ch.charCodeAt(0);
  return (
    isIdentChar(ch) ||
    c === 58 || // :
    c === 45 || // -
    c === 46    // .
  );
}

function tokenizeXml(code) {
  const tokens = [];
  const n = code.length;
  let i = 0;

  while (i < n) {
    if (code[i] === "<") {
      if (code.startsWith("<!--", i)) {
        const close = code.indexOf("-->", i + 4);
        const end = close === -1 ? n : close + 3;
        tokens.push({ type: "comment", start: i, end });
        i = end;
        if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
        continue;
      }

      if (code.startsWith("<![CDATA[", i)) {
        const close = code.indexOf("]]>", i + 9);
        const end = close === -1 ? n : close + 3;
        tokens.push({ type: "string", start: i, end });
        i = end;
        if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
        continue;
      }

      const isPi = code[i + 1] === "?";
      const isDecl = !isPi && code[i + 1] === "!";
      let cursor = i + (isPi || isDecl || code[i + 1] === "/" ? 2 : 1);
      if (cursor < n && (isIdentStart(code[cursor]) || code[cursor] === ":")) {
        let nameEnd = cursor + 1;
        while (nameEnd < n && isXmlNameChar(code[nameEnd])) nameEnd++;
        tokens.push({
          type: isPi || isDecl ? "attr" : "keyword",
          start: isPi ? i : cursor,
          end: nameEnd,
        });
        cursor = nameEnd;
        if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      }

      while (cursor < n) {
        const c = code[cursor];
        if (c === "<") break;
        if (isPi && c === "?" && code[cursor + 1] === ">") {
          tokens.push({ type: "attr", start: cursor, end: cursor + 2 });
          cursor += 2;
          if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
          break;
        }
        if (c === ">") {
          cursor++;
          break;
        }
        if (c === '"' || c === "'") {
          const close = code.indexOf(c, cursor + 1);
          const nl = code.indexOf("\n", cursor + 1);
          const end =
            close !== -1 && (nl === -1 || close < nl)
              ? close + 1
              : nl === -1
                ? n
                : nl;
          tokens.push({ type: "string", start: cursor, end });
          cursor = end;
          if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
          continue;
        }
        if (isIdentStart(c) || c === ":") {
          let attrEnd = cursor + 1;
          while (attrEnd < n && isXmlNameChar(code[attrEnd])) attrEnd++;
          tokens.push({ type: "type", start: cursor, end: attrEnd });
          cursor = attrEnd;
          if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
          continue;
        }
        cursor++;
      }
      i = cursor;
      continue;
    }

    if (code[i] === "&") {
      const semi = code.indexOf(";", i + 1);
      if (semi !== -1 && semi - i <= 16 && /^&(?:#x?[0-9a-fA-F]+|[A-Za-z][\w.-]*);$/.test(code.slice(i, semi + 1))) {
        tokens.push({ type: "number", start: i, end: semi + 1 });
        i = semi + 1;
        if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
        continue;
      }
    }

    i++;
  }

  return tokens;
}

function tokenizeJson(code) {
  const tokens = [];
  const n = code.length;
  let i = 0;

  while (i < n) {
    const ch = code[i];
    if (ch === " " || ch === "\t" || ch === "\n" || ch === "\r") {
      i++;
      continue;
    }

    if (ch === '"') {
      const end = scanQuoted(code, i, '"', false);
      let lookahead = end;
      while (lookahead < n && /\s/.test(code[lookahead])) lookahead++;
      const isKey = code[lookahead] === ":";
      tokens.push({ type: isKey ? "type" : "string", start: i, end });
      i = end;
      if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      continue;
    }

    if (isDigit(ch) || (ch === "-" && i + 1 < n && isDigit(code[i + 1]))) {
      let end = i + (ch === "-" ? 2 : 1);
      while (end < n && /[0-9.eE+-]/.test(code[end])) {
        if ((code[end] === "+" || code[end] === "-") && code[end - 1] !== "e" && code[end - 1] !== "E") {
          break;
        }
        end++;
      }
      tokens.push({ type: "number", start: i, end });
      i = end;
      if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      continue;
    }

    if (isIdentStart(ch)) {
      let end = i + 1;
      while (end < n && isIdentChar(code[end])) end++;
      const word = code.slice(i, end);
      if (word === "true" || word === "false" || word === "null") {
        tokens.push({ type: "keyword", start: i, end });
        if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      }
      i = end;
      continue;
    }

    i++;
  }

  return tokens;
}

function tokenizeCode(code, lang) {
  if (lang === "xml") return tokenizeXml(code);
  if (lang === "json") return tokenizeJson(code);
  const spec = LANG_SYNTAX[lang];
  if (!spec) return null;
  const tokens = [];
  const n = code.length;
  let i = 0;

  while (i < n) {
    const ch = code[i];
    const next = i + 1 < n ? code[i + 1] : "";

    if (ch === " " || ch === "\t" || ch === "\n" || ch === "\r") {
      i++;
      continue;
    }

    if (spec.hashComment && ch === "#") {
      const nl = code.indexOf("\n", i);
      const end = nl === -1 ? n : nl;
      tokens.push({ type: "comment", start: i, end });
      i = end;
      if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      continue;
    }

    if (spec.hashDirective && ch === "#") {
      let lineStart = i === 0 || code[i - 1] === "\n";
      if (!lineStart) {
        let j = i - 1;
        while (j >= 0 && (code[j] === " " || code[j] === "\t")) j--;
        lineStart = j < 0 || code[j] === "\n";
      }
      if (lineStart && i + 1 < n && isIdentStart(code[i + 1])) {
        let end = i + 2;
        while (end < n && isIdentChar(code[end])) end++;
        tokens.push({ type: "attr", start: i, end });
        i = end;
        if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
        continue;
      }
    }

    if (spec.cComments !== false && ch === "/" && next === "/") {
      const nl = code.indexOf("\n", i);
      const end = nl === -1 ? n : nl;
      tokens.push({ type: "comment", start: i, end });
      i = end;
      if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      continue;
    }

    if (spec.cComments !== false && ch === "/" && next === "*") {
      const close = code.indexOf("*/", i + 2);
      const end = close === -1 ? n : close + 2;
      tokens.push({ type: "comment", start: i, end });
      i = end;
      if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      continue;
    }

    if (
      spec.rustAttrs &&
      ch === "#" &&
      (next === "[" || (next === "!" && code[i + 2] === "["))
    ) {
      let depth = 0;
      let end = i + (next === "!" ? 2 : 1);
      while (end < n) {
        if (code[end] === "\n") break;
        if (code[end] === "[") depth++;
        else if (code[end] === "]") {
          depth--;
          if (depth === 0) {
            end++;
            break;
          }
        }
        end++;
      }
      tokens.push({ type: "attr", start: i, end });
      i = end;
      if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      continue;
    }

    if (spec.csharpAttrs && ch === "[") {
      let j = i - 1;
      while (j >= 0 && (code[j] === " " || code[j] === "\t")) j--;
      if (j < 0 || code[j] === "\n") {
        const nl = code.indexOf("\n", i);
        const close = code.indexOf("]", i + 1);
        if (close !== -1 && (nl === -1 || close < nl) && isIdentStart(code[i + 1] || "")) {
          const end = close + 1;
          tokens.push({ type: "attr", start: i, end });
          i = end;
          if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
          continue;
        }
      }
    }

    if (spec.atAnnotation && ch === "@" && i + 1 < n && isIdentStart(next)) {
      let end = i + 2;
      while (end < n && (isIdentChar(code[end]) || code[end] === ".")) end++;
      tokens.push({ type: "attr", start: i, end });
      i = end;
      if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      continue;
    }

    if (spec.rustRawStrings && (ch === "r" || (ch === "b" && next === "r"))) {
      const prev = i > 0 ? code[i - 1] : "";
      if (!prev || !isIdentChar(prev)) {
        let j = i + (ch === "b" ? 2 : 1);
        let hashes = 0;
        while (j < n && code[j] === "#") {
          hashes++;
          j++;
        }
        if (j < n && code[j] === '"') {
          const closing = '"' + "#".repeat(hashes);
          const closeIdx = code.indexOf(closing, j + 1);
          const end = closeIdx === -1 ? n : closeIdx + closing.length;
          tokens.push({ type: "string", start: i, end });
          i = end;
          if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
          continue;
        }
      }
    }

    if (
      spec.csharpVerbatim &&
      ((ch === "@" && next === '"') ||
        (ch === "$" && next === "@" && code[i + 2] === '"'))
    ) {
      let end = i + (ch === "$" ? 3 : 2);
      while (end < n) {
        if (code[end] === '"') {
          if (code[end + 1] === '"') {
            end += 2;
            continue;
          }
          end++;
          break;
        }
        end++;
      }
      tokens.push({ type: "string", start: i, end });
      i = end;
      if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      continue;
    }

    if (
      spec.tripleQuotes &&
      (ch === '"' || (spec.singleQuoteString && ch === "'")) &&
      code[i + 1] === ch &&
      code[i + 2] === ch
    ) {
      const delim = ch.repeat(3);
      let end = i + 3;
      while (end < n) {
        if (code[end] === "\\") {
          end += 2;
          continue;
        }
        if (code.startsWith(delim, end)) {
          end += 3;
          break;
        }
        end++;
      }
      tokens.push({ type: "string", start: i, end });
      i = end;
      if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      continue;
    }

    if (spec.backtick && ch === "`") {
      let end = i + 1;
      while (end < n) {
        if (spec.backtick === "string" && code[end] === "\\") {
          end += 2;
          continue;
        }
        if (code[end] === "`") {
          end++;
          break;
        }
        end++;
      }
      tokens.push({ type: spec.backtick, start: i, end });
      i = end;
      if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      continue;
    }

    if (ch === '"') {
      const end = scanQuoted(code, i, '"', false);
      tokens.push({ type: "string", start: i, end });
      i = end;
      if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      continue;
    }

    if (ch === "'") {
      if (spec.singleQuoteString) {
        const end = scanQuoted(code, i, "'", false);
        tokens.push({ type: "string", start: i, end });
        i = end;
        if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
        continue;
      }
      if (next === "\\" && i + 3 < n && code[i + 3] === "'") {
        tokens.push({ type: "string", start: i, end: i + 4 });
        i += 4;
        if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
        continue;
      }
      if (next && next !== "\n" && next !== "'" && code[i + 2] === "'") {
        tokens.push({ type: "string", start: i, end: i + 3 });
        i += 3;
        if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
        continue;
      }
      if (lang === "rust" && next && isIdentStart(next)) {
        let end = i + 2;
        while (end < n && isIdentChar(code[end])) end++;
        tokens.push({ type: "attr", start: i, end });
        i = end;
        if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
        continue;
      }
      i++;
      continue;
    }

    if (isDigit(ch)) {
      let end = i + 1;
      while (end < n && /[0-9a-fA-FxXoObB_'.eE+-]/.test(code[end])) {
        if ((code[end] === "+" || code[end] === "-") && code[end - 1] !== "e" && code[end - 1] !== "E") {
          break;
        }
        end++;
      }
      while (end < n && /[uUlLfFdDmM]/.test(code[end])) end++;
      tokens.push({ type: "number", start: i, end });
      i = end;
      if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      continue;
    }

    if (isIdentStart(ch)) {
      let end = i + 1;
      while (end < n && isIdentChar(code[end])) end++;
      const word = code.slice(i, end);
      let kind = null;
      if (spec.keywords.has(word)) {
        kind = "keyword";
      } else {
        let lookahead = end;
        while (lookahead < n && (code[lookahead] === " " || code[lookahead] === "\t")) {
          lookahead++;
        }
        if (code[lookahead] === "(" && !spec.builtins.has(word)) {
          kind = "fn";
        } else if (spec.builtins.has(word) || isPascalCaseType(word)) {
          kind = "type";
        }
      }
      if (kind) {
        tokens.push({ type: kind, start: i, end });
        if (tokens.length >= MAX_HIGHLIGHT_TOKENS) return null;
      }
      i = end;
      continue;
    }

    i++;
  }

  return tokens;
}

function syncCssHighlights() {
  if (typeof CSS === "undefined" || !CSS.highlights || typeof Highlight !== "function") {
    return;
  }
  for (const bucket of HIGHLIGHT_BUCKETS) {
    const hl = new Highlight();
    for (const targetMap of highlightStore.values()) {
      const ranges = targetMap.get(bucket);
      if (ranges) {
        for (const range of ranges) hl.add(range);
      }
    }
    if (hl.size > 0) {
      CSS.highlights.set(`tok-${bucket}`, hl);
    } else {
      CSS.highlights.delete(`tok-${bucket}`);
    }
  }
}

function applyCodeHighlight(codeEl, code, lang) {
  highlightStore.delete(codeEl);
  codeEl.textContent = code;

  if (!code || !lang || code.length > MAX_HIGHLIGHT_CHARS) {
    syncCssHighlights();
    return false;
  }

  const tokens = tokenizeCode(code, lang);
  if (!tokens) {
    syncCssHighlights();
    return false;
  }
  if (tokens.length === 0) {
    syncCssHighlights();
    return true;
  }

  if (typeof CSS !== "undefined" && CSS.highlights && typeof Highlight === "function") {
    const walker = document.createTreeWalker(codeEl, NodeFilter.SHOW_TEXT);
    const textNode = walker.nextNode();
    if (!textNode) {
      syncCssHighlights();
      return true;
    }

    const groups = new Map();
    for (const bucket of HIGHLIGHT_BUCKETS) groups.set(bucket, []);
    for (const tok of tokens) {
      const range = new Range();
      range.setStart(textNode, tok.start);
      range.setEnd(textNode, tok.end);
      groups.get(tok.type).push(range);
    }
    highlightStore.set(codeEl, groups);
    syncCssHighlights();
    return true;
  }

  // Fallback for browsers without CSS Custom Highlight API: textContent spans only.
  const frag = document.createDocumentFragment();
  let cursor = 0;
  for (const tok of tokens) {
    if (tok.start > cursor) {
      frag.append(document.createTextNode(code.slice(cursor, tok.start)));
    }
    const span = document.createElement("span");
    span.className = `tok-${tok.type}`;
    span.textContent = code.slice(tok.start, tok.end);
    frag.append(span);
    cursor = tok.end;
  }
  if (cursor < code.length) {
    frag.append(document.createTextNode(code.slice(cursor)));
  }
  codeEl.replaceChildren(frag);
  return true;
}

function renderCodeText(code, lang, { resetScroll = false } = {}) {
  applyCodeHighlight(generatedCode, code, lang);
  if (resetScroll) {
    codeOutput.scrollTo(0, 0);
  }
}

function createHighlightedEditor(textarea, codeEl, lang) {
  const stack = textarea.closest(".editor-stack");
  const backdrop = codeEl.parentElement;

  function syncScroll() {
    if (!backdrop) return;
    backdrop.scrollTop = textarea.scrollTop;
    backdrop.scrollLeft = textarea.scrollLeft;
  }

  function refresh() {
    const raw = textarea.value;
    if (!raw || raw.length > MAX_HIGHLIGHT_CHARS) {
      stack?.classList.remove("is-highlighted");
      highlightStore.delete(codeEl);
      codeEl.textContent = "";
      syncCssHighlights();
      return;
    }
    const display = raw.endsWith("\n") ? `${raw} ` : raw;
    const highlighted = applyCodeHighlight(codeEl, display, lang);
    if (highlighted) {
      stack?.classList.add("is-highlighted");
      syncScroll();
    } else {
      stack?.classList.remove("is-highlighted");
      codeEl.textContent = "";
    }
  }

  textarea.addEventListener("scroll", syncScroll, { passive: true });
  textarea.addEventListener("input", refresh);
  return { refresh, syncScroll };
}

const xsdEditor = createHighlightedEditor(xsdInput, xsdHighlight, "xml");
const xmlEditor = createHighlightedEditor(xmlInput, xmlHighlight, "xml");
const jsonEditor = createHighlightedEditor(jsonInput, jsonHighlight, "json");

function populateVariantSelect(lang) {
  const options = VARIANTS[lang] || [];
  variantSelect.replaceChildren();
  for (const item of options) {
    const opt = document.createElement("option");
    opt.value = item.value;
    opt.textContent = item.label;
    variantSelect.appendChild(opt);
  }
  const chosen = activeVariants[lang];
  const valid = chosen && options.some((item) => item.value === chosen);
  const selectedValue = valid ? chosen : options[0]?.value || "";
  if (selectedValue) {
    variantSelect.value = selectedValue;
    activeVariants[lang] = selectedValue;
  }
}

function renderFileTabs() {
  fileTabs.replaceChildren();
  if (activeFiles.length <= 1) {
    fileTabs.hidden = true;
    return;
  }
  fileTabs.hidden = false;
  activeFiles.forEach((file, idx) => {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "file-chip";
    btn.textContent = file.name;
    btn.setAttribute("aria-pressed", String(idx === activeFileIndex));
    btn.addEventListener("click", () => {
      activeFileIndex = idx;
      activeFileName = file.name;
      renderFileTabs();
      renderActiveFile({ resetScroll: true });
    });
    fileTabs.appendChild(btn);
  });
}

function renderActiveFile({ resetScroll = false } = {}) {
  const current = activeFiles[activeFileIndex];
  const lang = activeTab === "transcode" ? "rust" : activeTab;
  renderCodeText(current ? current.content : "", lang, { resetScroll });
}

function formatStats(stats) {
  const rootsCount = Array.isArray(stats.roots) ? stats.roots.length : 0;
  return `${stats.types} types (${stats.structs} structs, ${stats.enums} enums, ${rootsCount} roots)`;
}

function checkXsdLimit() {
  const xsd = xsdInput.value;
  const over = utf8BytesExceeds(xsd, MAX_XSD_BYTES);
  if (!over) return true;

  const kib = (over / 1024).toFixed(1);
  const msg = `Schema exceeds browser playground limit (256 KiB; got ${kib} KiB). Use the native CLI for multi-megabyte schemas: polyxml generate`;
  irStats.textContent = `Too large (${kib} KiB / 256 KiB max)`;
  compileTiming.textContent = "";
  activeFiles = [];
  activeFileIndex = 0;
  activeFileName = "";
  cachedStatsXsd = null;
  renderFileTabs();
  renderCodeText(
    "// Browser playground guardrail: schema exceeds 256 KiB.\n" +
      "//\n" +
      "// PolyXML compiles 8+ MB schemas natively via CLI without\n" +
      "// browser memory caps:\n" +
      `//   polyxml generate --lang ${activeTab === "transcode" ? "rust" : activeTab} schema.xsd\n`,
    activeTab === "transcode" ? "rust" : activeTab,
    { resetScroll: true }
  );
  showError(msg);
  return false;
}

function checkTranscodeLimit(text, label) {
  const over = utf8BytesExceeds(text, MAX_TRANSCODE_BYTES);
  if (!over) return true;
  const kib = (over / 1024).toFixed(1);
  showError(`${label} exceeds browser playground limit (512 KiB; got ${kib} KiB). Use polyxml CLI for large payloads.`);
  return false;
}

function renderCompileFailure(msg, lang) {
  activeFiles = [];
  renderFileTabs();
  if (msg.includes("Unique Particle Attribution")) {
    irStats.textContent = "UPA violation caught";
    const hasPresetFix = UPA_AMBIGUOUS_RE.test(xsdInput.value);
    showError(msg, { showUpaFix: hasPresetFix });
    renderCodeText(
      "// PolyXML intentionally rejected this schema before codegen.\n" +
        "//\n" +
        "// Why: Under W3C XSD 1.0 Unique Particle Attribution (UPA), when a\n" +
        "// parser sees <Item>, it cannot decide without lookahead whether\n" +
        "// to match the optional first particle (minOccurs=\"0\") or the\n" +
        "// required second particle.\n" +
        "//\n" +
        "// Fix: Collapse the adjacent elements into one bounded particle:\n" +
        "//   <xs:element name=\"Item\" type=\"xs:string\" minOccurs=\"1\" maxOccurs=\"2\"/>\n" +
        (hasPresetFix
          ? "//\n// Click \"Apply valid UPA fix\" above to repair and compile.\n"
          : ""),
      lang,
      { resetScroll: true }
    );
  } else {
    irStats.textContent = "Compile error";
    renderCodeText("", lang, { resetScroll: true });
    showError(msg);
  }
}

function runCompile({ resetScroll = false } = {}) {
  if (!wasm) {
    if (wasmError) {
      irStats.textContent = "WASM module unavailable";
      renderCodeText(wasmError, "rust", { resetScroll });
    }
    return;
  }

  if (!checkXsdLimit()) return;

  if (activeTab === "transcode") {
    runXmlToJson();
    return;
  }

  const start = performance.now();
  try {
    const variant = activeVariants[activeTab] || variantSelect.value || undefined;
    const raw = wasm.compile_xsd(xsdInput.value, activeTab, variant);
    const elapsed = (performance.now() - start).toFixed(1);
    const parsed = JSON.parse(raw);

    showError("");
    const summary = formatStats(parsed.stats);
    cachedStatsXsd = xsdInput.value;
    cachedStatsSummary = summary;
    irStats.textContent = summary;
    compileTiming.textContent = `${elapsed} ms`;

    activeFiles = parsed.files || [];
    const preservedIdx = activeFileName
      ? activeFiles.findIndex((f) => f.name === activeFileName)
      : -1;
    if (preservedIdx !== -1) {
      activeFileIndex = preservedIdx;
    } else if (activeFileIndex >= activeFiles.length) {
      activeFileIndex = 0;
    }
    activeFileName = activeFiles[activeFileIndex]?.name || "";
    renderFileTabs();
    renderActiveFile({ resetScroll });
  } catch (err) {
    const elapsed = (performance.now() - start).toFixed(1);
    compileTiming.textContent = `${elapsed} ms`;
    cachedStatsXsd = null;
    renderCompileFailure(String(err), activeTab);
  }
}

function refreshTranscodeIrStats() {
  if (!wasm) return;
  const xsd = xsdInput.value;
  if (cachedStatsXsd === xsd && cachedStatsSummary) {
    irStats.textContent = cachedStatsSummary;
    return;
  }
  try {
    const raw = wasm.compile_xsd(xsd, "rust", "zero-copy");
    const parsed = JSON.parse(raw);
    const summary = formatStats(parsed.stats);
    cachedStatsXsd = xsd;
    cachedStatsSummary = summary;
    irStats.textContent = summary;
  } catch {
    // Error handling is done by WasmSchema / compile check below.
  }
}

function detectXmlRootLocalName(xml) {
  const cleaned = xml
    .replace(/<\?[\s\S]*?\?>/g, "")
    .replace(/<!--[\s\S]*?-->/g, "")
    .replace(/<!DOCTYPE[\s\S]*?>/i, "");
  const match = cleaned.match(/<\s*([A-Za-z_][\w.-]*(?::[A-Za-z_][\w.-]*)?)/);
  if (!match) return undefined;
  const qname = match[1];
  const colon = qname.indexOf(":");
  return colon === -1 ? qname : qname.slice(colon + 1);
}

function withWasmSchema(fn, preferredRoot) {
  if (!wasm) return;
  if (!checkXsdLimit()) return;
  const xsd = xsdInput.value;
  let schema = null;
  try {
    try {
      schema = new wasm.WasmSchema(xsd, preferredRoot);
    } catch (firstErr) {
      if (preferredRoot && String(firstErr).includes("not found in schema")) {
        schema = new wasm.WasmSchema(xsd, undefined);
      } else {
        throw firstErr;
      }
    }
    fn(schema);
    showError("");
    refreshTranscodeIrStats();
  } catch (err) {
    const msg = String(err);
    cachedStatsXsd = null;
    if (msg.includes("Unique Particle Attribution")) {
      irStats.textContent = "UPA violation caught";
      showError(msg, { showUpaFix: UPA_AMBIGUOUS_RE.test(xsd) });
    } else {
      irStats.textContent = "Schema error";
      showError(msg);
    }
  } finally {
    if (schema) schema.free();
  }
}

function runXmlToJson() {
  if (!checkTranscodeLimit(xmlInput.value, "XML input")) return;
  if (!xmlInput.value.trim()) {
    refreshTranscodeIrStats();
    showError("Enter or paste an XML document to convert to JSON.");
    return;
  }
  const preferredRoot = detectXmlRootLocalName(xmlInput.value);
  withWasmSchema((schema) => {
    const out = schema.xml_to_json(encoder.encode(xmlInput.value));
    const parsed = JSON.parse(decoder.decode(out));
    jsonInput.value = JSON.stringify(parsed, null, 2);
    jsonEditor.refresh();
    announce("Converted XML to schema-typed JSON.");
  }, preferredRoot);
}

function runJsonToXml() {
  if (!checkTranscodeLimit(jsonInput.value, "JSON input")) return;
  if (!jsonInput.value.trim()) {
    refreshTranscodeIrStats();
    showError("Enter or paste JSON to convert to XML.");
    return;
  }
  try {
    JSON.parse(jsonInput.value);
  } catch (err) {
    showError(`Invalid JSON: ${err.message}`);
    return;
  }
  const preferredRoot = detectXmlRootLocalName(xmlInput.value);
  withWasmSchema((schema) => {
    const payload = encoder.encode(jsonInput.value);
    const out =
      typeof schema.json_to_xml_pretty === "function"
        ? schema.json_to_xml_pretty(payload, 2)
        : schema.json_to_xml(payload);
    xmlInput.value = decoder.decode(out);
    xmlEditor.refresh();
    announce("Converted JSON to XML.");
  }, preferredRoot);
}

function selectTab(nextId, { focusTab = false, scroll = true } = {}) {
  activeTab = nextId;
  let selectedBtn = null;
  for (const tab of tabs) {
    const isSelected = tab.dataset.tab === nextId;
    tab.ariaSelected = String(isSelected);
    tab.toggleAttribute("focusgroupstart", isSelected);
    if (!Object.hasOwn(HTMLElement.prototype, "focusGroup")) {
      tab.tabIndex = isSelected ? 0 : -1;
    }
    if (isSelected) selectedBtn = tab;
  }

  if (nextId === "transcode") {
    codePanel.hidden = "until-found";
    transcodePanel.hidden = false;
    xmlEditor.syncScroll();
    jsonEditor.syncScroll();
    runXmlToJson();
  } else {
    transcodePanel.hidden = "until-found";
    codePanel.hidden = false;
    if (selectedBtn) {
      codePanel.setAttribute("aria-labelledby", selectedBtn.id);
    }
    populateVariantSelect(nextId);
    activeFileIndex = 0;
    activeFileName = "";
    runCompile({ resetScroll: true });
  }

  if (selectedBtn && scroll) {
    try {
      selectedBtn.scrollIntoView({
        block: "nearest",
        inline: "nearest",
        container: "nearest",
      });
    } catch {
      selectedBtn.scrollIntoView({ block: "nearest", inline: "nearest" });
    }
    if (focusTab) selectedBtn.focus();
  } else if (selectedBtn && focusTab) {
    selectedBtn.focus();
  }
}

function loadPreset(key) {
  const preset = PRESETS[key] || PRESETS.iso20022;
  presetSelect.value = key in PRESETS ? key : "iso20022";
  xsdInput.value = preset.xsd;
  xmlInput.value = preset.xml;
  jsonInput.value = "";
  xsdEditor.refresh();
  xmlEditor.refresh();
  jsonEditor.refresh();
  activeFileIndex = 0;
  activeFileName = "";
  runCompile({ resetScroll: true });
}

function setupScrollFadeFallback() {
  if (CSS.supports("animation-timeline", "scroll()")) return;
  const first = tablist.firstElementChild;
  const last = tablist.lastElementChild;
  if (!first || !last) return;
  const observer = new IntersectionObserver(
    (entries) => {
      for (const { target, intersectionRatio } of entries) {
        tablist.classList.toggle(
          target === first ? "more-start" : "more-end",
          intersectionRatio < 1
        );
      }
    },
    { root: tablist, threshold: 1, rootMargin: "1px" }
  );
  observer.observe(first);
  observer.observe(last);
}

for (const tab of tabs) {
  tab.addEventListener("click", () => selectTab(tab.dataset.tab));
  tab.addEventListener("focus", () => {
    if (tab.dataset.tab !== activeTab) {
      selectTab(tab.dataset.tab);
    }
  });
}

if (!Object.hasOwn(HTMLElement.prototype, "focusGroup")) {
  tablist.addEventListener("keydown", (event) => {
    const idx = tabs.indexOf(document.activeElement);
    if (idx === -1) return;
    let nextIdx = -1;
    if (event.key === "ArrowRight") nextIdx = (idx + 1) % tabs.length;
    else if (event.key === "ArrowLeft") nextIdx = (idx - 1 + tabs.length) % tabs.length;
    else if (event.key === "Home") nextIdx = 0;
    else if (event.key === "End") nextIdx = tabs.length - 1;
    if (nextIdx !== -1) {
      event.preventDefault();
      selectTab(tabs[nextIdx].dataset.tab, { focusTab: true });
    }
  });
}

for (const panel of [codePanel, transcodePanel]) {
  panel.addEventListener("beforematch", () => {
    if (panel === transcodePanel) {
      selectTab("transcode");
    } else if (activeTab === "transcode") {
      selectTab("rust");
    }
  });
}

presetSelect.addEventListener("change", () => {
  loadPreset(presetSelect.value);
  clearUrlHash();
});

resetBtn.addEventListener("click", () => {
  loadPreset(presetSelect.value);
  clearUrlHash();
  announce("Reset schema to selected preset.");
});

variantSelect.addEventListener("change", () => {
  activeVariants[activeTab] = variantSelect.value;
  runCompile({ resetScroll: true });
});

xsdInput.addEventListener("input", () => {
  clearTimeout(compileTimer);
  const delay = xsdInput.value.length > 16_384 ? 320 : 120;
  compileTimer = setTimeout(() => runCompile(), delay);
});

codeOutput.addEventListener("keydown", (event) => {
  if (
    (event.ctrlKey || event.metaKey) &&
    !event.altKey &&
    !event.shiftKey &&
    event.key.toLowerCase() === "a"
  ) {
    event.preventDefault();
    const sel = window.getSelection();
    if (!sel) return;
    const range = document.createRange();
    range.selectNodeContents(generatedCode);
    sel.removeAllRanges();
    sel.addRange(range);
  }
});

async function handleCopyCommand(button) {
  const current = activeFiles[activeFileIndex];
  const text = current ? current.content : generatedCode.innerText.trim();
  if (!text) {
    reportCopyStatus(button, false, "Nothing to copy");
    return;
  }
  try {
    await writeToClipboard(text);
    reportCopyStatus(button, true, "Copied");
  } catch {
    reportCopyStatus(button, false, "Copy failed");
  }
}

document.addEventListener(
  "command",
  (event) => {
    if (event.command === "--copy" && event.source === copyCodeBtn) {
      handleCopyCommand(event.source);
    }
  },
  true
);

if (!("CommandEvent" in window)) {
  copyCodeBtn.addEventListener("click", () => handleCopyCommand(copyCodeBtn));
}

shareBtn.addEventListener("click", async () => {
  const token = encodeShareState({
    preset: presetSelect.value,
    tab: activeTab,
    variant: activeVariants[activeTab] || undefined,
    xsd: xsdInput.value,
    xml: xmlInput.value,
  });
  if (!token) {
    showError(
      "Schema + XML payload is too large for a shareable URL (limit: 16 KB encoded)."
    );
    reportCopyStatus(shareBtn, false, "Too large for URL");
    return;
  }
  const nextUrl = `${location.pathname}${location.search}#s=${token}`;
  history.replaceState(null, "", nextUrl);
  try {
    await writeToClipboard(location.href);
    reportCopyStatus(shareBtn, true, "Copied link");
  } catch {
    reportCopyStatus(shareBtn, true, "URL updated");
  }
});

xmlToJsonBtn.addEventListener("click", runXmlToJson);
jsonToXmlBtn.addEventListener("click", runJsonToXml);
fixUpaBtn.addEventListener("click", () => {
  if (UPA_AMBIGUOUS_RE.test(xsdInput.value)) {
    xsdInput.value = xsdInput.value.replace(
      UPA_AMBIGUOUS_RE,
      '$1<xs:element name="Item" type="xs:string" minOccurs="1" maxOccurs="2"/>'
    );
    xsdEditor.refresh();
    runCompile({ resetScroll: true });
    announce("Applied UPA fix and compiled schema cleanly.");
  }
});

function applyStateFromHash() {
  const shared = decodeShareState(location.hash);
  if (shared && typeof shared.xsd === "string") {
    if (shared.preset && shared.preset in PRESETS) {
      presetSelect.value = shared.preset;
    }
    xsdInput.value = shared.xsd;
    xmlInput.value =
      typeof shared.xml === "string" ? shared.xml : PRESETS.iso20022.xml;
    jsonInput.value = "";
    xsdEditor.refresh();
    xmlEditor.refresh();
    jsonEditor.refresh();
    if (shared.tab && (shared.tab in VARIANTS || shared.tab === "transcode")) {
      activeTab = shared.tab;
      if (
        shared.variant &&
        shared.tab in VARIANTS &&
        VARIANTS[shared.tab].some((v) => v.value === shared.variant)
      ) {
        activeVariants[shared.tab] = shared.variant;
      }
    }
    return true;
  }
  return false;
}

window.addEventListener("hashchange", () => {
  if (applyStateFromHash()) {
    selectTab(activeTab, { scroll: false });
  }
});

setupScrollFadeFallback();

async function init() {
  if (!applyStateFromHash()) {
    presetSelect.value = "iso20022";
    xsdInput.value = PRESETS.iso20022.xsd;
    xmlInput.value = PRESETS.iso20022.xml;
    xsdEditor.refresh();
    xmlEditor.refresh();
    jsonEditor.refresh();
  }

  populateVariantSelect(activeTab === "transcode" ? "rust" : activeTab);
  selectTab(activeTab, { scroll: false });

  try {
    const mod = await import("./pkg/polyxml_wasm.js");
    await mod.default();
    wasm = mod;
    runCompile({ resetScroll: true });
    announce("PolyXML WebAssembly compiler ready.");
  } catch (err) {
    wasmError =
      "// Build the playground WASM bundle first:\n" +
      "// wasm-pack build crates/polyxml-wasm --target web --release --out-dir ../../docs/playground/pkg -- --features codegen\n\n" +
      String(err);
    irStats.textContent = "WASM bundle not found";
    renderCodeText(wasmError, "rust", { resetScroll: true });
  }
}

init();
