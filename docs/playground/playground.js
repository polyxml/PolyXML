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

const presetSelect = document.getElementById("preset-select");
const shareBtn = document.getElementById("share-btn");
const resetBtn = document.getElementById("reset-btn");
const xsdInput = document.getElementById("xsd-input");
const irStats = document.getElementById("ir-stats");
const statusLive = document.getElementById("status-live");
const errorBanner = document.getElementById("error-banner");

const tablist = document.querySelector(".tablist");
const tabs = Array.from(tablist.querySelectorAll('[role="tab"]'));
const codePanel = document.getElementById("panel-code");
const transcodePanel = document.getElementById("panel-transcode");
const variantSelect = document.getElementById("variant-select");
const fileTabs = document.getElementById("file-tabs");
const compileTiming = document.getElementById("compile-timing");
const copyCodeBtn = document.getElementById("copy-code-btn");
const generatedCode = document.getElementById("generated-code");

const xmlInput = document.getElementById("xml-input");
const jsonInput = document.getElementById("json-input");
const xmlToJsonBtn = document.getElementById("xml-to-json-btn");
const jsonToXmlBtn = document.getElementById("json-to-xml-btn");

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });

let wasm = null;
let wasmError = null;
let activeTab = "rust";
let activeFiles = [];
let activeFileIndex = 0;
let activeVariants = {};
let compileTimer = null;

function announce(message) {
  statusLive.textContent = message;
}

function showError(message) {
  if (!message) {
    errorBanner.textContent = "";
    errorBanner.hidden = true;
    return;
  }
  errorBanner.textContent = message;
  errorBanner.hidden = false;
}

function encodeShareState(state) {
  const bytes = encoder.encode(JSON.stringify(state));
  let bin = "";
  for (const byte of bytes) {
    bin += String.fromCharCode(byte);
  }
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
}

function decodeShareState(hash) {
  if (!hash || !hash.startsWith("#s=")) return null;
  try {
    let b64 = hash.slice(3).replace(/-/g, "+").replace(/_/g, "/");
    while (b64.length % 4) b64 += "=";
    const bin = atob(b64);
    const bytes = Uint8Array.from(bin, (c) => c.charCodeAt(0));
    return JSON.parse(decoder.decode(bytes));
  } catch {
    return null;
  }
}

function populateVariantSelect(lang) {
  const options = VARIANTS[lang] || [];
  variantSelect.replaceChildren();
  for (const item of options) {
    const opt = document.createElement("option");
    opt.value = item.value;
    opt.textContent = item.label;
    variantSelect.appendChild(opt);
  }
  const chosen = activeVariants[lang] || (options[0] && options[0].value) || "";
  if (chosen) variantSelect.value = chosen;
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
      renderFileTabs();
      renderActiveFile();
    });
    fileTabs.appendChild(btn);
  });
}

function renderActiveFile() {
  const current = activeFiles[activeFileIndex];
  generatedCode.textContent = current ? current.content : "";
}

function runCompile() {
  if (!wasm) {
    if (wasmError) {
      irStats.textContent = "WASM module unavailable";
      generatedCode.textContent = wasmError;
    }
    return;
  }

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
    const stats = parsed.stats;
    const rootsCount = Array.isArray(stats.roots) ? stats.roots.length : 0;
    irStats.textContent = `${stats.types} types (${stats.structs} structs, ${stats.enums} enums, ${rootsCount} roots)`;
    compileTiming.textContent = `${elapsed} ms`;

    activeFiles = parsed.files || [];
    if (activeFileIndex >= activeFiles.length) {
      activeFileIndex = 0;
    }
    renderFileTabs();
    renderActiveFile();
  } catch (err) {
    const elapsed = (performance.now() - start).toFixed(1);
    compileTiming.textContent = `${elapsed} ms`;
    irStats.textContent = "Compile error";
    activeFiles = [];
    renderFileTabs();
    generatedCode.textContent = "";
    showError(String(err));
  }
}

function withWasmSchema(fn) {
  if (!wasm) return;
  let schema = null;
  try {
    schema = new wasm.WasmSchema(xsdInput.value, undefined);
    fn(schema);
    showError("");
  } catch (err) {
    showError(String(err));
  } finally {
    if (schema) schema.free();
  }
}

function runXmlToJson() {
  withWasmSchema((schema) => {
    const out = schema.xml_to_json(encoder.encode(xmlInput.value));
    const parsed = JSON.parse(decoder.decode(out));
    jsonInput.value = JSON.stringify(parsed, null, 2);
    announce("Converted XML to schema-typed JSON.");
  });
}

function runJsonToXml() {
  withWasmSchema((schema) => {
    const out = schema.json_to_xml(encoder.encode(jsonInput.value));
    xmlInput.value = decoder.decode(out);
    announce("Converted JSON to XML.");
  });
}

function selectTab(nextId, { focusTab = false } = {}) {
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
    codePanel.removeAttribute("tabindex");
    transcodePanel.hidden = false;
    transcodePanel.tabIndex = 0;
    runXmlToJson();
  } else {
    transcodePanel.hidden = "until-found";
    transcodePanel.removeAttribute("tabindex");
    codePanel.hidden = false;
    codePanel.tabIndex = 0;
    if (selectedBtn) {
      codePanel.setAttribute("aria-labelledby", selectedBtn.id);
    }
    populateVariantSelect(nextId);
    activeFileIndex = 0;
    runCompile();
  }

  if (selectedBtn) {
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
  }
}

function loadPreset(key) {
  const preset = PRESETS[key] || PRESETS.iso20022;
  presetSelect.value = key in PRESETS ? key : "iso20022";
  xsdInput.value = preset.xsd;
  xmlInput.value = preset.xml;
  jsonInput.value = "";
  activeFileIndex = 0;
  runCompile();
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
  history.replaceState(null, "", "#");
});

resetBtn.addEventListener("click", () => {
  loadPreset(presetSelect.value);
  history.replaceState(null, "", "#");
  announce("Reset schema to selected preset.");
});

variantSelect.addEventListener("change", () => {
  activeVariants[activeTab] = variantSelect.value;
  runCompile();
});

xsdInput.addEventListener("input", () => {
  clearTimeout(compileTimer);
  compileTimer = setTimeout(runCompile, 120);
});

copyCodeBtn.addEventListener("click", async () => {
  const text = generatedCode.textContent || "";
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    const prev = copyCodeBtn.textContent;
    copyCodeBtn.textContent = "Copied";
    announce("Generated code copied to clipboard.");
    setTimeout(() => {
      copyCodeBtn.textContent = prev;
    }, 1400);
  } catch {
    announce("Copy failed; select code and copy manually.");
  }
});

shareBtn.addEventListener("click", async () => {
  const token = encodeShareState({
    preset: presetSelect.value,
    tab: activeTab,
    variant: activeVariants[activeTab] || undefined,
    xsd: xsdInput.value,
    xml: xmlInput.value,
  });
  const nextUrl = `${location.pathname}${location.search}#s=${token}`;
  history.replaceState(null, "", nextUrl);
  try {
    await navigator.clipboard.writeText(location.href);
    const prev = shareBtn.textContent;
    shareBtn.textContent = "Copied link";
    announce("Shareable URL copied to clipboard.");
    setTimeout(() => {
      shareBtn.textContent = prev;
    }, 1500);
  } catch {
    announce("Updated URL hash with current playground state.");
  }
});

xmlToJsonBtn.addEventListener("click", runXmlToJson);
jsonToXmlBtn.addEventListener("click", runJsonToXml);

setupScrollFadeFallback();

async function init() {
  const shared = decodeShareState(location.hash);
  if (shared && typeof shared.xsd === "string") {
    if (shared.preset && shared.preset in PRESETS) presetSelect.value = shared.preset;
    xsdInput.value = shared.xsd;
    xmlInput.value = typeof shared.xml === "string" ? shared.xml : PRESETS.iso20022.xml;
    if (shared.tab && (shared.tab in VARIANTS || shared.tab === "transcode")) {
      activeTab = shared.tab;
      if (shared.variant && shared.tab in VARIANTS) {
        activeVariants[shared.tab] = shared.variant;
      }
    }
  } else {
    presetSelect.value = "iso20022";
    xsdInput.value = PRESETS.iso20022.xsd;
    xmlInput.value = PRESETS.iso20022.xml;
  }

  populateVariantSelect(activeTab === "transcode" ? "rust" : activeTab);
  selectTab(activeTab);

  try {
    const mod = await import("./pkg/polyxml_wasm.js");
    await mod.default();
    wasm = mod;
    selectTab(activeTab);
    announce("PolyXML WebAssembly compiler ready.");
  } catch (err) {
    wasmError =
      "// Build the playground WASM bundle first:\n" +
      "// wasm-pack build crates/polyxml-wasm --target web --release --out-dir ../../docs/playground/pkg -- --features codegen\n\n" +
      String(err);
    irStats.textContent = "WASM bundle not found";
    generatedCode.textContent = wasmError;
  }
}

init();
