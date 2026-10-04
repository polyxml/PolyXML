#!/usr/bin/env bash
# ==============================================================================
# scripts/verify_codegen.sh - PolyXML End-to-End Multi-Target Smoke Test
# ==============================================================================
# Compiles a representative XSD schema across all 7 target languages and backends
# into an ephemeral directory to verify CLI operation and file emission.
# ==============================================================================
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo -e "${BLUE}==> Building polyxml CLI binary...${NC}"
cargo build -p polyxml-cli --quiet
POLYXML_BIN="${CARGO_TARGET_DIR:-${REPO_ROOT}/target}/debug/polyxml"

TMP_DIR="$(mktemp -d -t polyxml-smoke-XXXXXX)"
trap 'rm -rf "${TMP_DIR}"' EXIT

SCHEMA_FILE="${TMP_DIR}/test_schema.xsd"
cat <<'EOF' > "${SCHEMA_FILE}"
<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:polyxml:smoke" xmlns="urn:polyxml:smoke" elementFormDefault="qualified">
  <xs:simpleType name="Status">
    <xs:restriction base="xs:string">
      <xs:enumeration value="Pending"/>
      <xs:enumeration value="Active"/>
      <xs:enumeration value="Suspended"/>
    </xs:restriction>
  </xs:simpleType>
  <xs:complexType name="TreeNode">
    <xs:sequence>
      <xs:element name="id" type="xs:int"/>
      <xs:element name="status" type="Status"/>
      <xs:element name="children" type="TreeNode" minOccurs="0" maxOccurs="unbounded"/>
    </xs:sequence>
  </xs:complexType>
  <xs:complexType name="SmokeModel">
    <xs:sequence>
      <xs:element name="title" type="xs:string"/>
      <xs:element name="root" type="TreeNode"/>
      <xs:choice>
        <xs:element name="email" type="xs:string"/>
        <xs:element name="phone" type="xs:string"/>
      </xs:choice>
    </xs:sequence>
    <xs:attribute name="version" type="xs:string" use="required"/>
  </xs:complexType>
</xs:schema>
EOF

echo -e "${BLUE}==> Testing code generation across all 7 target ecosystems...${NC}"

# 1. Rust (zero-copy + rkyv + perfect-hash dispatch)
echo -n "  • Rust (--feature zero-copy,rkyv,phf)... "
"${POLYXML_BIN}" generate --lang rust --feature zero-copy,rkyv,phf --out "${TMP_DIR}/rs" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/rs/test_schema.rs"
grep -q "ELEMENT_DISPATCH" "${TMP_DIR}/rs/test_schema.rs"
echo -e "${GREEN}OK${NC}"

# 2. Python (dataclass & pydantic)
echo -n "  • Python (dataclasses)... "
"${POLYXML_BIN}" generate --lang python --backend dataclasses --out "${TMP_DIR}/py_data" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/py_data/test_schema.py"
echo -e "${GREEN}OK${NC}"

echo -n "  • Python (pydantic-v2)... "
"${POLYXML_BIN}" generate --lang python --backend pydantic-v2 --out "${TMP_DIR}/py_pydantic" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/py_pydantic/test_schema.py"
echo -e "${GREEN}OK${NC}"

# 3. C++20 (headers, modules, glaze)
echo -n "  • C++20 (headers + glaze)... "
"${POLYXML_BIN}" generate --lang cpp --backend glaze --out "${TMP_DIR}/cpp_hdr" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/cpp_hdr/test_schema.hpp"
echo -e "${GREEN}OK${NC}"

echo -n "  • C++20 (modules)... "
"${POLYXML_BIN}" generate --lang cpp --mode modules --out "${TMP_DIR}/cpp_mod" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/cpp_mod/test_schema.cppm"
echo -e "${GREEN}OK${NC}"

# 4. Java 22+ (standard & jackson)
echo -n "  • Java 22+ (standard records)... "
"${POLYXML_BIN}" generate --lang java --package com.example.smoke --out "${TMP_DIR}/java_std" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/java_std/SmokeModel.java"
echo -e "${GREEN}OK${NC}"

echo -n "  • Java 22+ (jackson)... "
"${POLYXML_BIN}" generate --lang java --backend jackson --package com.example.smoke --out "${TMP_DIR}/java_jack" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/java_jack/SmokeModel.java"
echo -e "${GREEN}OK${NC}"

# Mutable Java models and direct codecs.
echo -n "  • Java (POJO + builder + direct codec)... "
"${POLYXML_BIN}" generate --lang java --style pojo --feature builder,direct-codec --out "${TMP_DIR}/java_pojo" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/java_pojo/TreeNodeCodec.java"
if command -v javac >/dev/null 2>&1; then
    javac -d "${TMP_DIR}/java_classes" "${TMP_DIR}/java_pojo/"*.java
fi
echo -e "${GREEN}OK${NC}"

# 5. TypeScript (zod, valibot, typebox)
echo -n "  • TypeScript (zod)... "
"${POLYXML_BIN}" generate --lang typescript --backend zod --out "${TMP_DIR}/ts_zod" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/ts_zod/test_schema.ts"
echo -e "${GREEN}OK${NC}"

echo -n "  • TypeScript (valibot)... "
"${POLYXML_BIN}" generate --lang typescript --backend valibot --out "${TMP_DIR}/ts_valibot" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/ts_valibot/test_schema.ts"
echo -e "${GREEN}OK${NC}"

echo -n "  • TypeScript (typebox)... "
"${POLYXML_BIN}" generate --lang typescript --backend typebox --out "${TMP_DIR}/ts_typebox" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/ts_typebox/test_schema.ts"
echo -e "${GREEN}OK${NC}"

# 6. Go (standard, easyjson, sonic)
echo -n "  • Go (standard)... "
"${POLYXML_BIN}" generate --lang go --package smoke --out "${TMP_DIR}/go_std" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/go_std/test_schema.go"
echo -e "${GREEN}OK${NC}"

echo -n "  • Go (sonic)... "
"${POLYXML_BIN}" generate --lang go --backend sonic --package smoke --out "${TMP_DIR}/go_sonic" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/go_sonic/test_schema.go"
echo -e "${GREEN}OK${NC}"

echo -n "  • Go (easyjson)... "
"${POLYXML_BIN}" generate --lang go --backend easyjson --package smoke --out "${TMP_DIR}/go_easyjson" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/go_easyjson/test_schema.go"
echo -e "${GREEN}OK${NC}"

# 7. C# (class, struct, source-gen)
echo -n "  • C# (record class)... "
"${POLYXML_BIN}" generate --lang csharp --namespace Smoke --out "${TMP_DIR}/cs_class" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/cs_class/TestSchema.cs"
echo -e "${GREEN}OK${NC}"

echo -n "  • C# (record struct + source-gen)... "
"${POLYXML_BIN}" generate --lang csharp --style record-struct --backend source-gen --namespace Smoke --out "${TMP_DIR}/cs_struct" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/cs_struct/TestSchema.cs"
echo -e "${GREEN}OK${NC}"


echo -n "  • C# (mutable class + source-gen)... "
"${POLYXML_BIN}" generate --lang csharp --style class --backend source-gen --out "${TMP_DIR}/cs_class" "${SCHEMA_FILE}" >/dev/null
test -f "${TMP_DIR}/cs_class/TestSchema.cs"
echo -e "${GREEN}OK${NC}"
echo -e "${GREEN}✨ Multi-target smoke verification passed completely across all 7 ecosystems!${NC}"
