use std::fs;
use std::process::Command;
use tempfile::tempdir;

use polyxml_cli::config::WorkspaceManifest;

#[test]
fn test_cli_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .arg("--help")
        .output()
        .expect("Failed to execute binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Modern XSD-to-code generator"));
    assert!(stdout.contains("generate"));
    assert!(stdout.contains("build"));
    assert!(stdout.contains("validate"));
}

#[test]
fn test_cli_generate_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["generate", "--help"])
        .output()
        .expect("Failed to execute binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--lang"));
    assert!(stdout.contains("--out"));
    assert!(stdout.contains("--strict-facets"));
    assert!(stdout.contains("--dry-run"));
    assert!(stdout.contains("--format"));
}

#[test]
fn test_cli_validate_valid_and_invalid() {
    let dir = tempdir().unwrap();
    let valid_xsd = dir.path().join("test.xsd");
    fs::write(
        &valid_xsd,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test">
            <xs:element name="Hello" type="xs:string"/>
        </xs:schema>"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["validate", valid_xsd.to_str().unwrap()])
        .output()
        .expect("Failed to execute validate");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Valid schema"));
    assert!(stdout.contains("urn:test"));

    // Test non-existent schema
    let missing_xsd = dir.path().join("missing.xsd");
    let bad_output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["validate", missing_xsd.to_str().unwrap()])
        .output()
        .expect("Failed to execute validate");

    assert!(!bad_output.status.success());
}

#[test]
fn test_cli_generate_dry_run() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("order.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:orders">
            <xs:complexType name="Order">
                <xs:sequence>
                    <xs:element name="Id" type="xs:string"/>
                    <xs:element name="Amount" type="xs:decimal"/>
                </xs:sequence>
            </xs:complexType>
            <xs:element name="PurchaseOrder" type="Order"/>
        </xs:schema>"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["generate", "--dry-run", schema_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute generate --dry-run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dry run completed successfully. No files written."));
    assert!(stdout.contains("Namespace: urn:orders"));
    assert!(stdout.contains("Root elements: 1"));
}

#[test]
fn test_cli_generate_multi_lang() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("sample.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:sample">
            <xs:complexType name="SampleItem">
                <xs:sequence>
                    <xs:element name="Name" type="xs:string"/>
                </xs:sequence>
            </xs:complexType>
        </xs:schema>"#,
    )
    .unwrap();

    let out_dir = dir.path().join("out");

    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            "--lang",
            "python",
            "--lang",
            "rust",
            "--out",
            out_dir.to_str().unwrap(),
            schema_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute generate");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Code generation complete."));

    assert!(out_dir.join("python/sample.py").exists());
    assert!(out_dir.join("rust/sample.rs").exists());
}

#[test]
fn test_polyxml_toml_manifest_build() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("contract.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:corp">
            <xs:element name="Agreement" type="xs:string"/>
        </xs:schema>"#,
    )
    .unwrap();

    let manifest_file = dir.path().join("polyxml.toml");
    let manifest_content = r#"
[workspace]
name = "contracts"
schemas = ["*.xsd"]
output_base_dir = "./dist"

[[generate]]
target = "python"
output = "py_models"
backend = "pydantic-v2"

[codegen.rust]
enabled = true
output = "rs_models"
zero_copy = true
"#;
    fs::write(&manifest_file, manifest_content).unwrap();

    // Test config parsing unit
    let parsed = WorkspaceManifest::from_file(&manifest_file).unwrap();
    assert_eq!(
        parsed.workspace.as_ref().unwrap().name.as_deref(),
        Some("contracts")
    );
    let targets = parsed.resolved_targets();
    assert_eq!(targets.len(), 2);
    let schemas = parsed.expand_schemas(dir.path()).unwrap();
    assert_eq!(schemas.len(), 1);

    // Test CLI build command
    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["build", "--config", manifest_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute build");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Build finished successfully."));
    assert!(dir.path().join("dist/py_models/contract.py").exists());
    assert!(dir.path().join("dist/rs_models/contract.rs").exists());
}

#[test]
fn test_cli_generate_python_backends() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("invoice.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="https://example.com/invoice">
            <xs:simpleType name="InvoiceCode">
                <xs:restriction base="xs:string">
                    <xs:minLength value="5"/>
                    <xs:maxLength value="10"/>
                </xs:restriction>
            </xs:simpleType>
            <xs:complexType name="Invoice">
                <xs:sequence>
                    <xs:element name="Code" type="InvoiceCode"/>
                    <xs:element name="Total" type="xs:decimal"/>
                    <xs:element name="Note" type="xs:string" minOccurs="0" nillable="true"/>
                </xs:sequence>
                <xs:attribute name="id" type="xs:int" use="required"/>
            </xs:complexType>
            <xs:element name="InvoiceDoc" type="Invoice"/>
        </xs:schema>"#,
    )
    .unwrap();

    // 1. Generate with dataclass backend
    let dc_out = dir.path().join("out_dataclass");
    let output_dc = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            "--lang",
            "python",
            "--backend",
            "dataclass",
            "--out",
            dc_out.to_str().unwrap(),
            schema_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute dataclass generate");

    assert!(output_dc.status.success());
    let dc_py = fs::read_to_string(dc_out.join("invoice.py")).unwrap();
    assert!(dc_py.contains("@dataclass(slots=True, kw_only=True)"));
    assert!(dc_py.contains("class Invoice:"));
    assert!(dc_py.contains("code: InvoiceCode = field("));
    assert!(dc_py.contains("total: Decimal = field("));
    assert!(dc_py.contains("note: str | None = field(default=None"));
    assert!(dc_py.contains("id: int = field("));
    assert!(dc_py.contains("type InvoiceDoc = Invoice"));

    // 2. Generate with pydantic backend
    let pydantic_out = dir.path().join("out_pydantic");
    let output_pydantic = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            "--lang",
            "python",
            "--backend",
            "pydantic",
            "--out",
            pydantic_out.to_str().unwrap(),
            schema_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute pydantic generate");

    assert!(output_pydantic.status.success());
    let pyd_py = fs::read_to_string(pydantic_out.join("invoice.py")).unwrap();
    assert!(pyd_py.contains("class Invoice(BaseModel):"));
    assert!(pyd_py.contains("model_config = ConfigDict(defer_build=True, populate_by_name=True)"));
    assert!(
        pyd_py.contains("type InvoiceCode = Annotated[str, Field(min_length=5, max_length=10)]")
    );
    assert!(pyd_py.contains("code: InvoiceCode = Field(..., alias=\"Code\", serialization_alias=\"Code\", json_schema_extra={\"type\": \"Element\", \"name\": \"Code\", \"json_name\": \"Code\", \"namespace\": \"https://example.com/invoice\"})"));
    assert!(pyd_py.contains("total: Decimal = Field(..., alias=\"Total\", serialization_alias=\"Total\", json_schema_extra={\"type\": \"Element\", \"name\": \"Total\", \"json_name\": \"Total\", \"namespace\": \"https://example.com/invoice\"})"));
    assert!(pyd_py.contains("note: str | None = Field(default=None, alias=\"Note\", serialization_alias=\"Note\", json_schema_extra={\"type\": \"Element\", \"name\": \"Note\", \"json_name\": \"Note\", \"namespace\": \"https://example.com/invoice\", \"nillable\": True})"));
}

#[test]
fn test_cli_generate_rust_zero_copy_and_owned() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("customer.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:customers">
            <xs:simpleType name="Status">
                <xs:restriction base="xs:string">
                    <xs:enumeration value="Active"/>
                    <xs:enumeration value="Suspended"/>
                </xs:restriction>
            </xs:simpleType>
            <xs:complexType name="Customer">
                <xs:sequence>
                    <xs:element name="Name" type="xs:string"/>
                    <xs:element name="Status" type="Status"/>
                    <xs:element name="Balance" type="xs:decimal"/>
                </xs:sequence>
                <xs:attribute name="id" type="xs:int" use="required"/>
            </xs:complexType>
            <xs:element name="CustomerRecord" type="Customer"/>
        </xs:schema>"#,
    )
    .unwrap();

    // 1. Generate zero-copy Rust
    let zc_out = dir.path().join("out_rust_zc");
    let output_zc = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            "--lang",
            "rust",
            "--zero-copy",
            "--out",
            zc_out.to_str().unwrap(),
            schema_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute rust zero-copy generate");

    assert!(output_zc.status.success());
    assert!(zc_out.join("mod.rs").exists());
    let zc_rs = fs::read_to_string(zc_out.join("customer.rs")).unwrap();
    assert!(zc_rs.contains("use std::borrow::Cow;"));
    assert!(zc_rs.contains("pub struct Customer<'a>"));
    assert!(zc_rs.contains("pub name: Cow<'a, str>"));
    assert!(zc_rs.contains("pub enum Status"));
    assert!(zc_rs.contains("impl Status"));
    assert!(zc_rs.contains("pub type CustomerRecord<'a> = Customer<'a>;"));

    // 2. Generate owned Rust
    let owned_out = dir.path().join("out_rust_owned");
    let output_owned = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            "--lang",
            "rust",
            "--zero-copy",
            "false",
            "--out",
            owned_out.to_str().unwrap(),
            schema_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute rust owned generate");

    assert!(output_owned.status.success());
    let owned_rs = fs::read_to_string(owned_out.join("customer.rs")).unwrap();
    assert!(owned_rs.contains("pub struct Customer {"));
    assert!(owned_rs.contains("pub name: String"));
    assert!(!owned_rs.contains("Cow<'a"));
    assert!(owned_rs.contains("pub type CustomerRecord = Customer;"));
}

#[test]
fn test_cli_typescript_generation() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("customer.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:crm">
            <xs:simpleType name="Status">
                <xs:restriction base="xs:string">
                    <xs:enumeration value="active"/>
                    <xs:enumeration value="suspended"/>
                </xs:restriction>
            </xs:simpleType>
            <xs:complexType name="Customer">
                <xs:sequence>
                    <xs:element name="name" type="xs:string"/>
                    <xs:element name="status" type="Status"/>
                    <xs:element name="tag" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>
                </xs:sequence>
                <xs:attribute name="id" type="xs:int" use="required"/>
            </xs:complexType>
            <xs:element name="CustomerRecord" type="Customer"/>
        </xs:schema>"#,
    )
    .unwrap();

    // 1. Generate standard TypeScript
    let ts_out = dir.path().join("out_ts");
    let output_ts = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            "--lang",
            "ts",
            "--out",
            ts_out.to_str().unwrap(),
            schema_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute typescript generate");

    assert!(output_ts.status.success());
    assert!(ts_out.join("index.ts").exists());
    let ts_code = fs::read_to_string(ts_out.join("customer.ts")).unwrap();
    assert!(ts_code.contains("export interface Customer {"));
    assert!(ts_code.contains("id: number;"));
    assert!(ts_code.contains("name: string;"));
    assert!(ts_code.contains("status: Status;"));
    assert!(ts_code.contains("tag: string[];"));
    assert!(ts_code.contains("export const Status = {"));
    assert!(ts_code.contains("export type Status = (typeof Status)[keyof typeof Status];"));
    assert!(ts_code.contains("export type CustomerRecord = Customer;"));

    // Verify TypeScript compiles cleanly with tsc --strict
    let tsc_check = Command::new("tsc")
        .args([
            "--noEmit",
            "--strict",
            "--target",
            "es2022",
            ts_out.join("customer.ts").to_str().unwrap(),
        ])
        .output();
    if let Ok(tsc_out) = tsc_check {
        assert!(
            tsc_out.status.success(),
            "tsc failed on customer.ts: {}\nstdout: {}",
            String::from_utf8_lossy(&tsc_out.stderr),
            String::from_utf8_lossy(&tsc_out.stdout)
        );
    }

    // 2. Generate TypeScript with Zod schemas
    let zod_out = dir.path().join("out_ts_zod");
    let output_zod = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            "--lang",
            "ts",
            "--backend",
            "zod",
            "--out",
            zod_out.to_str().unwrap(),
            schema_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute typescript zod generate");

    assert!(output_zod.status.success());
    let zod_code = fs::read_to_string(zod_out.join("customer.ts")).unwrap();
    assert!(zod_code.contains("import { z } from \"zod\";"));
    assert!(zod_code.contains("export const StatusSchema = z.enum([\"active\", \"suspended\"]);"));
    assert!(zod_code.contains("export const CustomerSchema = z.object({"));
    assert!(zod_code.contains("id: z.number().int(),"));
    assert!(zod_code.contains("name: z.string(),"));
    assert!(zod_code.contains("status: StatusSchema,"));
    assert!(zod_code.contains("tag: z.array(z.string()),"));

    // Verify with tsc using ambient declaration for zod
    let stub_file = zod_out.join("zod_stub.d.ts");
    fs::write(
        &stub_file,
        "declare module \"zod\" { export const z: any; export namespace z { export type ZodType<T = any> = any; } }\n",
    )
    .unwrap();

    let tsc_zod_check = Command::new("tsc")
        .args([
            "--noEmit",
            "--strict",
            "--target",
            "es2022",
            stub_file.to_str().unwrap(),
            zod_out.join("customer.ts").to_str().unwrap(),
        ])
        .output();
    if let Ok(tsc_out) = tsc_zod_check {
        assert!(
            tsc_out.status.success(),
            "tsc failed on customer.ts with zod: {}\nstdout: {}",
            String::from_utf8_lossy(&tsc_out.stderr),
            String::from_utf8_lossy(&tsc_out.stdout)
        );
    }
}

#[test]
fn test_cli_java_generation() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("customer.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:crm">
            <xs:simpleType name="Status">
                <xs:restriction base="xs:string">
                    <xs:enumeration value="active"/>
                    <xs:enumeration value="suspended"/>
                </xs:restriction>
            </xs:simpleType>
            <xs:complexType name="Customer">
                <xs:sequence>
                    <xs:element name="name" type="xs:string"/>
                    <xs:element name="status" type="Status"/>
                    <xs:element name="tag" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>
                </xs:sequence>
                <xs:attribute name="id" type="xs:int" use="required"/>
            </xs:complexType>
            <xs:element name="CustomerRecord" type="Customer"/>
        </xs:schema>"#,
    )
    .unwrap();

    let java_out = dir.path().join("out_java");
    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            "--lang",
            "java",
            "--package",
            "com.enterprise.crm",
            "--out",
            java_out.to_str().unwrap(),
            schema_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute java generate");

    assert!(output.status.success());
    assert!(java_out.join("Customer.java").exists());
    assert!(java_out.join("Status.java").exists());

    let customer_code = fs::read_to_string(java_out.join("Customer.java")).unwrap();
    assert!(customer_code.contains("package com.enterprise.crm;"));
    assert!(customer_code.contains("public record Customer("));
    assert!(customer_code.contains("int id"));
    assert!(customer_code.contains("String name,"));
    assert!(customer_code.contains("Status status,"));
    assert!(customer_code.contains("java.util.List<String> tag"));

    let status_code = fs::read_to_string(java_out.join("Status.java")).unwrap();
    assert!(status_code.contains("package com.enterprise.crm;"));
    assert!(status_code.contains("public enum Status {"));
    assert!(status_code.contains("ACTIVE(\"active\"),"));
    assert!(status_code.contains("SUSPENDED(\"suspended\");"));

    // Verify Java compilation with javac -Werror
    let javac_check = Command::new("javac")
        .args([
            "-Werror",
            java_out.join("Status.java").to_str().unwrap(),
            java_out.join("Customer.java").to_str().unwrap(),
        ])
        .output();

    if let Ok(javac_out) = javac_check {
        assert!(
            javac_out.status.success(),
            "javac failed on generated Java 22 files: {}\nstdout: {}",
            String::from_utf8_lossy(&javac_out.stderr),
            String::from_utf8_lossy(&javac_out.stdout)
        );
    }
}

#[test]
fn test_cli_cpp_generation() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("crm.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:enterprise:crm">
            <xs:simpleType name="Status">
                <xs:restriction base="xs:string">
                    <xs:enumeration value="active"/>
                    <xs:enumeration value="suspended"/>
                </xs:restriction>
            </xs:simpleType>
            <xs:complexType name="Customer">
                <xs:sequence>
                    <xs:element name="name" type="xs:string"/>
                    <xs:element name="email" type="xs:string" minOccurs="0"/>
                    <xs:element name="status" type="Status"/>
                    <xs:element name="tag" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>
                </xs:sequence>
                <xs:attribute name="id" type="xs:int" use="required"/>
            </xs:complexType>
            <xs:element name="CustomerRecord" type="Customer"/>
        </xs:schema>"#,
    )
    .unwrap();

    let cpp_out = dir.path().join("out_cpp");
    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            "--lang",
            "cpp",
            "--namespace",
            "enterprise::crm",
            "--out",
            cpp_out.to_str().unwrap(),
            schema_file.to_str().unwrap(),
            "--format",
        ])
        .output()
        .expect("Failed to execute cpp generate");

    assert!(
        output.status.success(),
        "polyxml generate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let header_file = cpp_out.join("crm.hpp");
    assert!(header_file.exists(), "crm.hpp was not generated");

    let cpp_code = fs::read_to_string(&header_file).unwrap();
    assert!(cpp_code.contains("namespace enterprise::crm {"));
    assert!(cpp_code.contains("struct Customer {"));
    assert!(cpp_code.contains("std::string name"));
    assert!(cpp_code.contains("std::optional<std::string> email"));
    assert!(cpp_code.contains("Status status"));
    assert!(cpp_code.contains("std::vector<std::string> tag"));
    assert!(cpp_code.contains("std::int32_t id"));
    assert!(cpp_code.contains("enum class Status {"));
    assert!(cpp_code.contains("Active,"));
    assert!(cpp_code.contains("Suspended,"));
    assert!(cpp_code.contains("using CustomerRecord = Customer;"));
    assert!(cpp_code.contains("operator==") && cpp_code.contains("default"));

    // Verify C++20 compilation and execution with g++
    let driver_cpp = dir.path().join("driver.cpp");
    fs::write(
        &driver_cpp,
        r#"
#include "crm.hpp"
#include <cassert>
#include <iostream>

int main() {
    using namespace enterprise::crm;

    Customer c1{
        .name = "Acme Corp",
        .email = "info@acme.com",
        .status = Status::Active,
        .tag = {"enterprise", "partner"},
        .id = 100
    };

    Customer c2{
        .name = "Acme Corp",
        .email = "info@acme.com",
        .status = Status::Active,
        .tag = {"enterprise", "partner"},
        .id = 100
    };

    Customer c3{
        .name = "Beta LLC",
        .email = std::nullopt,
        .status = Status::Suspended,
        .tag = {},
        .id = 101
    };

    assert(c1 == c2);
    assert(!(c1 == c3));
    assert(to_string(Status::Active) == "active");
    assert(to_string(Status::Suspended) == "suspended");

    std::cout << "E2E C++20 driver passed!" << std::endl;
    return 0;
}
"#,
    )
    .unwrap();

    let out_bin = dir.path().join("driver_bin");
    let compile_status = Command::new("g++")
        .args([
            "-std=c++20",
            "-Wall",
            "-Wextra",
            "-Wpedantic",
            "-Werror",
            "-I",
            cpp_out.to_str().unwrap(),
            driver_cpp.to_str().unwrap(),
            "-o",
            out_bin.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute g++");

    assert!(
        compile_status.success(),
        "g++ compilation of generated crm.hpp failed"
    );

    let run_status = Command::new(&out_bin)
        .status()
        .expect("Failed to run compiled C++ binary");
    assert!(run_status.success(), "C++ test driver failed execution");
}

#[test]
fn test_cli_cpp_modules_and_glaze() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("crm.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:enterprise:crm">
            <xs:simpleType name="Status">
                <xs:restriction base="xs:string">
                    <xs:enumeration value="active"/>
                    <xs:enumeration value="suspended"/>
                </xs:restriction>
            </xs:simpleType>
            <xs:complexType name="Customer">
                <xs:sequence>
                    <xs:element name="name" type="xs:string"/>
                    <xs:element name="status" type="Status"/>
                </xs:sequence>
                <xs:attribute name="id" type="xs:int" use="required"/>
            </xs:complexType>
        </xs:schema>"#,
    )
    .unwrap();

    // 1. Test --mode modules
    let modules_out = dir.path().join("cpp_modules");
    let bin = env!("CARGO_BIN_EXE_polyxml");
    let output = Command::new(bin)
        .args([
            "generate",
            "--lang",
            "cpp",
            "--mode",
            "modules",
            "--package",
            "enterprise::crm",
            "--out",
            modules_out.to_str().unwrap(),
            schema_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute cpp modules generate");

    assert!(
        output.status.success(),
        "polyxml generate --mode modules failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cppm_file = modules_out.join("crm.cppm");
    assert!(cppm_file.exists(), "crm.cppm was not generated");
    let cppm_code = fs::read_to_string(&cppm_file).unwrap();
    assert!(cppm_code.contains("export module crm;"));
    assert!(cppm_code.contains("export namespace enterprise::crm {"));
    assert!(cppm_code.contains("struct Customer {"));

    // 2. Test --backend glaze
    let glaze_out = dir.path().join("cpp_glaze");
    let output2 = Command::new(bin)
        .args([
            "generate",
            "--lang",
            "cpp",
            "--backend",
            "glaze",
            "--package",
            "enterprise::crm",
            "--out",
            glaze_out.to_str().unwrap(),
            schema_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute cpp glaze generate");

    assert!(
        output2.status.success(),
        "polyxml generate --backend glaze failed: {}",
        String::from_utf8_lossy(&output2.stderr)
    );
    let hpp_file = glaze_out.join("crm.hpp");
    assert!(hpp_file.exists(), "crm.hpp was not generated");
    let hpp_code = fs::read_to_string(&hpp_file).unwrap();
    assert!(hpp_code.contains("#include <glaze/glaze.hpp>"));
    assert!(hpp_code.contains("struct glz::meta<enterprise::crm::Customer> {"));
    assert!(hpp_code.contains("struct glz::meta<enterprise::crm::Status> {"));
    assert!(hpp_code.contains("\"active\", T::Active"));
    assert!(hpp_code.contains("\"name\", &T::name"));
}

#[test]
fn test_cli_go_generation() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("crm.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="https://example.com/crm">
    <xs:simpleType name="AccountTier">
        <xs:restriction base="xs:string">
            <xs:enumeration value="standard"/>
            <xs:enumeration value="premium"/>
            <xs:enumeration value="enterprise"/>
        </xs:restriction>
    </xs:simpleType>

    <xs:complexType name="Account">
        <xs:sequence>
            <xs:element name="name" type="xs:string"/>
            <xs:element name="tier" type="AccountTier"/>
            <xs:element name="balance" type="xs:decimal"/>
            <xs:element name="alias" type="xs:string" minOccurs="0"/>
            <xs:element name="tag" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>
        </xs:sequence>
        <xs:attribute name="id" type="xs:int" use="required"/>
    </xs:complexType>

    <xs:element name="AccountRecord" type="Account"/>
</xs:schema>"#,
    )
    .unwrap();

    let go_out = dir.path().join("out_go");
    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            "--lang",
            "go",
            "--package",
            "crm",
            "--out",
            go_out.to_str().unwrap(),
            schema_file.to_str().unwrap(),
            "--format",
        ])
        .output()
        .expect("Failed to execute Go generate");

    assert!(
        output.status.success(),
        "CLI generate --lang go failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated_file = go_out.join("crm.go");
    assert!(generated_file.exists(), "crm.go was not created");
    let go_code = fs::read_to_string(&generated_file).unwrap();
    println!("GO_CODE:\n{}", go_code);

    assert!(go_code.contains("package crm"));
    assert!(go_code.contains("type AccountTier string"));
    assert!(go_code.contains("AccountTierStandard"));
    assert!(go_code.contains("\"standard\""));
    assert!(go_code.contains("func (e AccountTier) IsValid() bool"));
    assert!(go_code.contains("type Account struct {"));
    assert!(go_code.contains("XMLName xml.Name") && go_code.contains("`json:\"-\"`"));
    assert!(
        go_code.contains("Name")
            && go_code.contains("xml:\"name\"")
            && go_code.contains("json:\"name\"")
    );
    assert!(
        go_code.contains("Tier")
            && go_code.contains("AccountTier")
            && go_code.contains("xml:\"tier\"")
            && go_code.contains("json:\"tier\"")
    );
    assert!(
        go_code.contains("Balance")
            && go_code.contains("float64")
            && go_code.contains("xml:\"balance\"")
            && go_code.contains("json:\"balance\"")
    );
    assert!(
        go_code.contains("Alias")
            && go_code.contains("*string")
            && go_code.contains("xml:\"alias,omitempty\"")
            && go_code.contains("json:\"alias,omitempty\"")
    );
    assert!(
        go_code.contains("Tag")
            && go_code.contains("[]string")
            && go_code.contains("xml:\"tag\"")
            && go_code.contains("json:\"tag\"")
    );
    assert!(
        go_code.contains("ID")
            && go_code.contains("int32")
            && go_code.contains("xml:\"id,attr\"")
            && go_code.contains("json:\"id\"")
    );
    assert!(go_code.contains("func (s Account) Validate() error"));

    // Write a Go test driver to verify with `go test` and `go vet`
    let driver_go = go_out.join("crm_test.go");
    fs::write(
        &driver_go,
        r#"package crm

import (
    "encoding/xml"
    "testing"
)

func TestAccountE2E(t *testing.T) {
    alias := "AcmeMain"
    acc := Account{
        Name:    "Acme Corp",
        Tier:    AccountTierPremium,
        Balance: 1250.50,
        Alias:   &alias,
        Tag:     []string{"b2b", "strategic"},
        ID:      1001,
    }

    if !acc.Tier.IsValid() {
        t.Fatalf("expected tier to be valid")
    }

    data, err := xml.MarshalIndent(acc, "", "  ")
    if err != nil {
        t.Fatalf("xml.Marshal failed: %v", err)
    }

    var decoded Account
    if err := xml.Unmarshal(data, &decoded); err != nil {
        t.Fatalf("xml.Unmarshal failed: %v", err)
    }

    if decoded.Name != "Acme Corp" || decoded.Tier != AccountTierPremium || decoded.ID != 1001 {
        t.Fatalf("mismatched decoded values: %+v", decoded)
    }
    if decoded.Alias == nil || *decoded.Alias != "AcmeMain" {
        t.Fatalf("mismatched alias: %+v", decoded.Alias)
    }
    if len(decoded.Tag) != 2 || decoded.Tag[0] != "b2b" {
        t.Fatalf("mismatched tags: %+v", decoded.Tag)
    }

    if err := decoded.Validate(); err != nil {
        t.Fatalf("validation failed: %v", err)
    }
}
"#,
    )
    .unwrap();

    let init_status = Command::new("go")
        .args(["mod", "init", "crm"])
        .current_dir(&go_out)
        .status()
        .expect("Failed to run go mod init");
    assert!(init_status.success(), "go mod init failed");

    let vet_status = Command::new("go")
        .args(["vet", "."])
        .current_dir(&go_out)
        .status()
        .expect("Failed to run go vet");
    assert!(vet_status.success(), "go vet failed on generated Go models");

    let test_status = Command::new("go")
        .args(["test", "-v", "."])
        .current_dir(&go_out)
        .status()
        .expect("Failed to run go test");
    assert!(
        test_status.success(),
        "go test failed on generated Go models"
    );
}

#[test]
fn test_cli_csharp_generation() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("customer.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="https://example.com/crm">
    <xs:simpleType name="AccountTier">
        <xs:restriction base="xs:string">
            <xs:enumeration value="standard"/>
            <xs:enumeration value="premium"/>
            <xs:enumeration value="enterprise"/>
        </xs:restriction>
    </xs:simpleType>

    <xs:complexType name="Account">
        <xs:sequence>
            <xs:element name="name" type="xs:string"/>
            <xs:element name="tier" type="AccountTier"/>
            <xs:element name="balance" type="xs:decimal"/>
            <xs:element name="alias" type="xs:string" minOccurs="0"/>
            <xs:element name="tag" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>
        </xs:sequence>
        <xs:attribute name="id" type="xs:int" use="required"/>
    </xs:complexType>

    <xs:element name="AccountRecord" type="Account"/>
</xs:schema>"#,
    )
    .unwrap();

    let cs_out = dir.path().join("out_cs");
    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            "--lang",
            "csharp",
            "--package",
            "Enterprise.Crm",
            "--out",
            cs_out.to_str().unwrap(),
            schema_file.to_str().unwrap(),
            "--format",
        ])
        .output()
        .expect("Failed to execute C# generate");

    assert!(
        output.status.success(),
        "CLI generate --lang csharp failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let generated_file = cs_out.join("Customer.cs");
    assert!(generated_file.exists(), "Customer.cs was not created");
    let cs_code = fs::read_to_string(&generated_file).unwrap();

    assert!(cs_code.contains("namespace Enterprise.Crm;"));
    assert!(cs_code.contains("public enum AccountTier"));
    assert!(cs_code.contains("[XmlEnum(\"standard\")]"));
    assert!(cs_code.contains("public static bool IsValid(this AccountTier value)"));
    assert!(cs_code.contains("public record Account("));
    assert!(
        cs_code.contains("XmlAttribute(\"id\")")
            && cs_code.contains("JsonPropertyName(\"id\")")
            && cs_code.contains("int Id")
    );
    assert!(
        cs_code.contains("XmlElement(\"name\")")
            && cs_code.contains("JsonPropertyName(\"name\")")
            && cs_code.contains("string Name")
    );
    assert!(
        cs_code.contains("XmlElement(\"tier\")")
            && cs_code.contains("JsonPropertyName(\"tier\")")
            && cs_code.contains("AccountTier Tier")
    );
    assert!(
        cs_code.contains("XmlElement(\"balance\")")
            && cs_code.contains("JsonPropertyName(\"balance\")")
            && cs_code.contains("decimal Balance")
    );
    assert!(
        cs_code.contains("XmlElement(\"alias\")")
            && cs_code.contains("JsonPropertyName(\"alias\")")
            && cs_code.contains("string? Alias")
    );
    assert!(
        cs_code.contains("XmlElement(\"tag\")")
            && cs_code.contains("JsonPropertyName(\"tag\")")
            && cs_code.contains("List<string>? Tag")
    );
    assert!(cs_code.contains("public Account() : this("));

    // Verify .NET build and test driver
    let app_dir = dir.path().join("cli_csharp_app");
    fs::create_dir_all(&app_dir).unwrap();
    let csproj = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>
</Project>"#;
    fs::write(app_dir.join("CrmCliApp.csproj"), csproj).unwrap();

    fs::copy(&generated_file, app_dir.join("Customer.cs")).unwrap();

    fs::write(
        app_dir.join("Program.cs"),
        r#"using System;
using System.Collections.Generic;
using System.IO;
using System.Xml.Serialization;
using Enterprise.Crm;

public class Program
{
    public static int Main()
    {
        var acc = new Account(
            Id: 999,
            Name: "Enterprise LLC",
            Tier: AccountTier.Enterprise,
            Balance: 5000.75m,
            Alias: "EntCorp",
            Tag: new List<string> { "b2b", "tier1" }
        );

        if (!acc.Tier.IsValid() || acc.Tier.ToXmlValue() != "enterprise")
        {
            Console.WriteLine("Tier enum methods failed");
            return 1;
        }

        var serializer = new XmlSerializer(typeof(Account));
        using var sw = new StringWriter();
        serializer.Serialize(sw, acc);
        var xml = sw.ToString();

        using var sr = new StringReader(xml);
        var decoded = (Account?)serializer.Deserialize(sr);
        if (decoded == null)
        {
            Console.WriteLine("Deserialization failed");
            return 1;
        }

        if (decoded.Id != 999 || decoded.Name != "Enterprise LLC" || decoded.Tier != AccountTier.Enterprise || decoded.Balance != 5000.75m)
        {
            Console.WriteLine("Account fields mismatch");
            return 1;
        }

        if (decoded.Alias != "EntCorp" || decoded.Tag == null || decoded.Tag.Count != 2)
        {
            Console.WriteLine("Alias or Tag mismatch");
            return 1;
        }

        Console.WriteLine("E2E C# CLI test passed successfully!");
        return 0;
    }
}
"#,
    )
    .unwrap();

    let build_status = Command::new("dotnet")
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
        .args(["build", "--warnaserror"])
        .current_dir(&app_dir)
        .status()
        .expect("Failed to run dotnet build");
    assert!(
        build_status.success(),
        "dotnet build failed on CLI generated C# files"
    );

    let run_status = Command::new("dotnet")
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
        .args(["run"])
        .current_dir(&app_dir)
        .status()
        .expect("Failed to run dotnet run");
    assert!(
        run_status.success(),
        "dotnet run failed on CLI generated C# files"
    );
}

#[test]
fn test_cli_transcode_bidirectional() {
    let dir = tempdir().unwrap();
    let xml_file = dir.path().join("input.xml");
    let json_file = dir.path().join("output.json");
    let roundtrip_xml_file = dir.path().join("roundtrip.xml");

    fs::write(
        &xml_file,
        r#"<service id="99" enabled="true"><name>API Gateway</name><port>8080</port><port>8443</port></service>"#,
    )
    .unwrap();

    let exe = env!("CARGO_BIN_EXE_polyxml");

    // 1. XML to JSON with --pretty
    let status = Command::new(exe)
        .args([
            "transcode",
            xml_file.to_str().unwrap(),
            "-o",
            json_file.to_str().unwrap(),
            "--pretty",
        ])
        .status()
        .expect("failed to execute polyxml transcode");
    assert!(status.success());

    let json_str = fs::read_to_string(&json_file).unwrap();
    assert!(json_str.contains("\"@id\": 99"));
    assert!(json_str.contains("\"@enabled\": true"));
    assert!(json_str.contains("\"name\": \"API Gateway\""));
    assert!(json_str.contains("\"port\": [\n      8080,\n      8443\n    ]"));

    // 2. JSON to XML
    let status2 = Command::new(exe)
        .args([
            "transcode",
            json_file.to_str().unwrap(),
            "-o",
            roundtrip_xml_file.to_str().unwrap(),
            "--pretty",
        ])
        .status()
        .expect("failed to execute polyxml transcode");
    assert!(status2.success());

    let roundtrip_str = fs::read_to_string(&roundtrip_xml_file).unwrap();
    assert!(
        roundtrip_str.contains("<service")
            && roundtrip_str.contains("id=\"99\"")
            && roundtrip_str.contains("enabled=\"true\"")
    );
    assert!(roundtrip_str.contains("<name>API Gateway</name>"));
    assert!(roundtrip_str.contains("<port>8080</port>"));
    assert!(roundtrip_str.contains("<port>8443</port>"));
}

#[test]
fn test_cli_csharp_source_gen_and_record_struct() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("customer.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:crm">
            <xs:complexType name="Customer">
                <xs:sequence>
                    <xs:element name="Id" type="xs:int"/>
                    <xs:element name="Name" type="xs:string"/>
                </xs:sequence>
            </xs:complexType>
        </xs:schema>"#,
    )
    .unwrap();

    let out_dir = dir.path().join("csharp_out");
    let status = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            schema_file.to_str().unwrap(),
            "--lang",
            "csharp",
            "--backend",
            "source-gen",
            "--style",
            "record-struct",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute generate");
    assert!(status.success());

    let cs_file = out_dir.join("Customer.cs");
    let content = fs::read_to_string(&cs_file).unwrap();
    assert!(content.contains("public readonly record struct Customer("));
    assert!(content.contains("[JsonSourceGenerationOptions(WriteIndented = true)]"));
    assert!(content.contains("[JsonSerializable(typeof(Customer))]"));
    assert!(content.contains("public partial class CustomerJsonContext : JsonSerializerContext"));
}

#[test]
fn test_cli_typescript_valibot_and_typebox_backend() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("item.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:store">
            <xs:complexType name="Item">
                <xs:sequence>
                    <xs:element name="Sku" type="xs:string"/>
                    <xs:element name="Price" type="xs:decimal"/>
                </xs:sequence>
            </xs:complexType>
        </xs:schema>"#,
    )
    .unwrap();

    // 1. Valibot
    let vali_dir = dir.path().join("ts_vali");
    let status_vali = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            schema_file.to_str().unwrap(),
            "--lang",
            "ts",
            "--backend",
            "valibot",
            "-o",
            vali_dir.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute generate with valibot");
    assert!(status_vali.success());

    let vali_content = fs::read_to_string(vali_dir.join("item.ts")).unwrap();
    assert!(vali_content.contains("import * as v from \"valibot\";"));
    assert!(vali_content.contains("export const ItemSchema = v.object({"));

    // 2. TypeBox
    let tb_dir = dir.path().join("ts_tb");
    let status_tb = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            schema_file.to_str().unwrap(),
            "--lang",
            "ts",
            "--backend",
            "typebox",
            "-o",
            tb_dir.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute generate with typebox");
    assert!(status_tb.success());

    let tb_content = fs::read_to_string(tb_dir.join("item.ts")).unwrap();
    assert!(tb_content.contains("import { Type, Static } from \"@sinclair/typebox\";"));
    assert!(tb_content.contains("export const ItemSchema = Type.Object({"));
}

#[test]
fn test_cli_go_easyjson_and_sonic_backend() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("service.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:service">
            <xs:complexType name="Config">
                <xs:sequence>
                    <xs:element name="Host" type="xs:string"/>
                    <xs:element name="Port" type="xs:int"/>
                </xs:sequence>
            </xs:complexType>
        </xs:schema>"#,
    )
    .unwrap();

    // 1. EasyJSON
    let easy_dir = dir.path().join("go_easy");
    let status_easy = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            schema_file.to_str().unwrap(),
            "--lang",
            "go",
            "--backend",
            "easyjson",
            "-o",
            easy_dir.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute generate with easyjson");
    assert!(status_easy.success());

    let easy_content = fs::read_to_string(easy_dir.join("service.go")).unwrap();
    assert!(easy_content.contains("//easyjson:json"));
    assert!(easy_content.contains("type Config struct {"));

    // 2. Sonic
    let sonic_dir = dir.path().join("go_sonic");
    let status_sonic = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            schema_file.to_str().unwrap(),
            "--lang",
            "go",
            "--backend",
            "sonic",
            "-o",
            sonic_dir.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute generate with sonic");
    assert!(status_sonic.success());

    let sonic_content = fs::read_to_string(sonic_dir.join("service.go")).unwrap();
    assert!(sonic_content.contains("sonic:\"Host\""));
    assert!(sonic_content.contains("sonic:\"Port\""));
}

#[test]
fn test_cli_rust_rkyv_feature() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("event.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:events">
            <xs:complexType name="Event">
                <xs:sequence>
                    <xs:element name="Id" type="xs:long"/>
                </xs:sequence>
            </xs:complexType>
        </xs:schema>"#,
    )
    .unwrap();

    let out_dir = dir.path().join("rust_out");
    let status = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            schema_file.to_str().unwrap(),
            "--lang",
            "rust",
            "--feature",
            "rkyv",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute generate with rkyv");
    assert!(status.success());

    let rs_content = fs::read_to_string(out_dir.join("event.rs")).unwrap();
    assert!(rs_content.contains("#[cfg_attr(feature = \"rkyv\", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]"));
    assert!(rs_content.contains("#[cfg_attr(feature = \"rkyv\", rkyv(check_bytes))]"));
}

#[test]
fn test_cli_rust_phf_feature() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("event.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:events">
            <xs:complexType name="Event">
                <xs:sequence>
                    <xs:element name="Id" type="xs:long"/>
                </xs:sequence>
            </xs:complexType>
        </xs:schema>"#,
    )
    .unwrap();

    let out_dir = dir.path().join("rust_out");
    let status = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            schema_file.to_str().unwrap(),
            "--lang",
            "rust",
            "--feature",
            "phf",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute generate with phf");
    assert!(status.success());

    let rs_content = fs::read_to_string(out_dir.join("event.rs")).unwrap();
    assert!(rs_content.contains("enum __EventElementId"));
    assert!(
        rs_content.contains(
            "static __EVENT_ELEMENT_DISPATCH: ::phf::Map<&'static str, __EventElementId>"
        ),
        "phf dispatch table missing:\n{rs_content}"
    );
    assert!(rs_content.contains("(\"Id\", __EventElementId::Id)"));
    assert!(rs_content.contains("Some(&__EventElementId::Id) => {"));
}

#[test]
fn test_cli_build_polyxml_manifest_with_new_features() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("model.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:app">
            <xs:complexType name="AppConfig">
                <xs:sequence>
                    <xs:element name="Name" type="xs:string"/>
                </xs:sequence>
            </xs:complexType>
        </xs:schema>"#,
    )
    .unwrap();

    let manifest_file = dir.path().join("polyxml.toml");
    fs::write(
        &manifest_file,
        r#"
[workspace]
name = "feature-test"
schemas = ["model.xsd"]

[[generate]]
target = "csharp"
output = "cs"
backend = "source-gen"
style = "record-struct"

[[generate]]
target = "ts"
output = "ts"
backend = "valibot"

[[generate]]
target = "go"
output = "go"
backend = "sonic"

[[generate]]
target = "rust"
output = "rs"
features = ["rkyv"]
"#,
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["build", "-c", manifest_file.to_str().unwrap()])
        .current_dir(dir.path())
        .status()
        .expect("Failed to execute polyxml build");
    assert!(status.success());

    // Verify C# output
    let cs = fs::read_to_string(dir.path().join("cs").join("Model.cs")).unwrap();
    assert!(cs.contains("record struct AppConfig"));
    assert!(cs.contains("[JsonSourceGenerationOptions"));

    // Verify TS output
    let ts = fs::read_to_string(dir.path().join("ts").join("model.ts")).unwrap();
    assert!(ts.contains("import * as v from \"valibot\";"));

    // Verify Go output
    let go = fs::read_to_string(dir.path().join("go").join("model.go")).unwrap();
    assert!(go.contains("sonic:\"Name\""));

    // Verify Rust output
    let rs = fs::read_to_string(dir.path().join("rs").join("model.rs")).unwrap();
    assert!(rs.contains("derive(rkyv::Archive"));
}

#[test]
fn test_cli_generate_custom_header_and_generated_tags() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("service.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
            <xs:complexType name="Config">
                <xs:sequence>
                    <xs:element name="host" type="xs:string"/>
                </xs:sequence>
            </xs:complexType>
        </xs:schema>"#,
    )
    .unwrap();

    let out_dir = dir.path().join("out_py");
    let header_text = "# Copyright (c) 2026 Acme Corp\n# ruff: noqa";

    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            "--lang",
            "python",
            "--custom-header",
            header_text,
            "--out",
            out_dir.to_str().unwrap(),
            schema_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute polyxml generate");

    assert!(
        output.status.success(),
        "Stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let py_file = out_dir.join("service.py");
    assert!(py_file.exists());

    let py_code = fs::read_to_string(&py_file).unwrap();
    assert!(py_code.starts_with(
        "# Copyright (c) 2026 Acme Corp\n# ruff: noqa\n\n# @generated by PolyXML Compiler"
    ));
    assert!(py_code.contains("class Config:"));
}

#[test]
fn test_cli_manifest_custom_header_workspace_and_target_override() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("schema.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
            <xs:complexType name="Server">
                <xs:sequence>
                    <xs:element name="port" type="xs:int"/>
                </xs:sequence>
            </xs:complexType>
        </xs:schema>"#,
    )
    .unwrap();

    let manifest_file = dir.path().join("polyxml.toml");
    fs::write(
        &manifest_file,
        r#"
[workspace]
schemas = ["schema.xsd"]
custom_header = "// Shared Workspace License Header"

[[generate]]
target = "rust"
output = "rs"

[[generate]]
target = "go"
output = "go"
custom_header = "// Target Specific Go Header"

[[generate]]
target = "csharp"
output = "cs"
"#,
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["build", "-c", manifest_file.to_str().unwrap()])
        .current_dir(dir.path())
        .status()
        .expect("Failed to execute polyxml build");
    assert!(status.success());

    // Rust inherits workspace custom_header
    let rs = fs::read_to_string(dir.path().join("rs").join("schema.rs")).unwrap();
    assert!(
        rs.starts_with("// Shared Workspace License Header\n\n// @generated by PolyXML Compiler")
    );

    // Go overrides with target-specific custom_header and has go-standard generated comments
    let go = fs::read_to_string(dir.path().join("go").join("schema.go")).unwrap();
    assert!(go.starts_with("// Target Specific Go Header\n\n// Code generated by PolyXML Compiler"));
    assert!(go.contains("// @generated by PolyXML Compiler"));

    // C# inherits workspace custom_header and has <auto-generated/>
    let cs = fs::read_to_string(dir.path().join("cs").join("Schema.cs")).unwrap();
    assert!(cs.starts_with("// Shared Workspace License Header\n\n// <auto-generated/>\n// @generated by PolyXML Compiler"));
}

#[test]
fn test_java_csharp_model_style_options_and_manifest() {
    let dir = tempdir().unwrap();
    let schema = dir.path().join("model.xsd");
    fs::write(&schema,r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:complexType name="Model"><xs:sequence><xs:element name="id" type="xs:string"/></xs:sequence></xs:complexType></xs:schema>"#).unwrap();
    let java = dir.path().join("java");
    let result = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            schema.to_str().unwrap(),
            "--lang",
            "java",
            "--style",
            "pojo",
            "--feature",
            "builder,direct-codec",
            "--out",
            java.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let model = fs::read_to_string(java.join("Model.java")).unwrap();
    assert!(model.contains("public class Model"));
    assert!(model.contains("ModelBuilder builder()"));
    assert!(java.join("ModelCodec.java").exists());
    for args in [
        vec!["--lang", "go", "--style", "pojo"],
        vec!["--lang", "csharp", "--feature", "builder"],
        vec!["--lang", "cs", "--feature", "zero-copy"],
        vec!["--lang", "java", "--backend", "unknown"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_polyxml"))
            .args([
                "generate",
                schema.to_str().unwrap(),
                "--out",
                dir.path().join("bad").to_str().unwrap(),
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(!result.status.success());
    }
    let manifest = dir.path().join("polyxml.toml");
    fs::write(
        &manifest,
        r#"[workspace]
schemas = ["model.xsd"]
[[generate]]
target = "java"
output = "generated-java"
style = "class"
features = ["builder", "direct-codec"]
[codegen.csharp]
output = "generated-csharp"
style = "pojo"
"#,
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["build", "--config", manifest.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(dir.path().join("generated-java/ModelCodec.java").exists());
    let cs = fs::read_to_string(dir.path().join("generated-csharp/Model.cs")).unwrap();
    assert!(cs.contains("public class Model"));
    assert!(cs.contains("get; set;"));
    let parsed: WorkspaceManifest = r#"[codegen.java]
style="pojo"
features=["builder", "direct-codec"]
"#
    .parse()
    .unwrap();
    let target = &parsed.resolved_targets()[0];
    assert_eq!(target.style.as_deref(), Some("pojo"));
    assert_eq!(
        target.features,
        vec!["builder".to_string(), "direct-codec".to_string()]
    );
}

#[test]
fn unified_options_fail_before_schema_io_including_dry_run() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("output");
    for (flags, expected) in [
        (
            vec!["--lang", "python", "--backend", "jackson"],
            "Supported backends: dataclass, pydantic",
        ),
        (vec!["--lang", "ts", "--backend", "zodd"], "backend 'zodd'"),
        (
            vec!["--lang", "python", "--feature", "rkyv"],
            "feature 'rkyv'",
        ),
        (
            vec!["--lang", "rust", "--feature", "builder"],
            "feature 'builder'",
        ),
        (
            vec![
                "--lang",
                "rust",
                "--zero-copy=false",
                "--feature",
                "zero-copy",
            ],
            "conflicts",
        ),
        (
            vec![
                "--lang",
                "python",
                "--backend",
                "pydantic",
                "--style",
                "dataclass",
            ],
            "requires the Python dataclass backend",
        ),
        (
            vec!["--lang", "python", "--style", "class"],
            "Supported styles",
        ),
        (
            vec!["--lang", "cs", "--style", "struct"],
            "Supported styles",
        ),
        (
            vec!["--lang", "java", "--lang", "python", "--feature", "builder"],
            "feature 'builder'",
        ),
        (vec!["--lang", "typo"], "Unknown target"),
    ] {
        for dry_run in [false, true] {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_polyxml"));
            cmd.args(["generate", "missing.xsd"])
                .args(&flags)
                .arg("--out")
                .arg(&out);
            if dry_run {
                cmd.arg("--dry-run");
            }
            let result = cmd.output().unwrap();
            let stderr = String::from_utf8_lossy(&result.stderr);
            assert!(!result.status.success());
            assert!(stderr.contains(expected), "{flags:?}: {stderr}");
            assert!(!out.exists());
        }
    }
}

#[test]
fn unified_manifest_forms_generate_identical_output_and_validate_early() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("model.xsd"), r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:complexType name="Item"><xs:sequence><xs:element name="name" type="xs:string"/></xs:sequence></xs:complexType></xs:schema>"#).unwrap();
    let manifest = dir.path().join("polyxml.toml");
    fs::write(
        &manifest,
        r#"
[workspace]
schemas = ["model.xsd"]
[[generate]]
target = "java"
output = "array"
backend = "standard"
style = "pojo"
features = ["builder", "direct-codec"]
[codegen.java]
output = "table"
backend = "standard"
style = "pojo"
features = ["builder", "direct-codec"]
"#,
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["build", "--config"])
        .arg(&manifest)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for file in ["Item.java", "ItemCodec.java"] {
        assert_eq!(
            fs::read(dir.path().join("array").join(file)).unwrap(),
            fs::read(dir.path().join("table").join(file)).unwrap()
        );
    }
    let model = fs::read_to_string(dir.path().join("array/Item.java")).unwrap();
    assert!(model.contains("class Item"));
    assert!(model.contains("builder()"));
    for section in ["[[generate]]\ntarget = 'python'", "[codegen.python]"] {
        fs::write(&manifest, format!("[workspace]\nschemas = ['missing.xsd']\n{section}\noutput = 'invalid'\nfeatures = ['builder']\n")).unwrap();
        for command in ["build", "generate"] {
            let result = Command::new(env!("CARGO_BIN_EXE_polyxml"))
                .args([command, "--dry-run", "--config"])
                .arg(&manifest)
                .output()
                .unwrap();
            assert!(!result.status.success());
            assert!(String::from_utf8_lossy(&result.stderr).contains("feature 'builder'"));
            assert!(!dir.path().join("invalid").exists());
        }
    }
}

#[test]
fn unified_help_shows_generation_options() {
    let result = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["generate", "--help"])
        .output()
        .unwrap();
    let help = String::from_utf8_lossy(&result.stdout);
    for flag in ["--backend", "--style", "--feature", "--zero-copy"] {
        assert!(help.contains(flag));
    }
}

#[test]
fn unified_supported_backends_and_styles_validate() {
    let dir = tempdir().unwrap();
    let schema = dir.path().join("model.xsd");
    fs::write(
        &schema,
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"/>"#,
    )
    .unwrap();
    for (lang, backends, styles) in [
        (
            "python",
            vec!["dataclass", "pydantic", "pydantic-v2"],
            vec!["dataclass"],
        ),
        ("rust", vec!["standard"], vec![]),
        (
            "ts",
            vec!["interfaces", "zod", "valibot", "typebox", "none"],
            vec![],
        ),
        (
            "java",
            vec!["standard", "jackson"],
            vec!["record", "pojo", "class"],
        ),
        (
            "c#",
            vec!["standard", "source-gen"],
            vec!["record", "record-class", "record-struct", "class", "pojo"],
        ),
        ("c++", vec!["standard", "glaze"], vec![]),
        ("go", vec!["standard", "easyjson", "sonic"], vec![]),
    ] {
        for (flag, values) in [("--backend", backends), ("--style", styles)] {
            for value in values {
                let result = Command::new(env!("CARGO_BIN_EXE_polyxml"))
                    .arg("generate")
                    .arg(&schema)
                    .args(["--lang", lang, flag, value, "--dry-run"])
                    .output()
                    .unwrap();
                assert!(
                    result.status.success(),
                    "{lang} {flag} {value}: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
            }
        }
    }
}

#[test]
fn manifest_generation_does_not_ignore_cli_overrides() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("polyxml.toml"), "").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .current_dir(dir.path())
        .args(["generate", "--feature", "builder"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("set target options in polyxml.toml"));
}

#[test]
fn python_features_respect_manifest_false_values_and_backend() {
    let dir = tempdir().unwrap();
    let schema = dir.path().join("model.xsd");
    fs::write(&schema, r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:complexType name="Item"><xs:sequence><xs:element name="name" type="xs:string"/></xs:sequence></xs:complexType></xs:schema>"#).unwrap();
    let manifest = dir.path().join("polyxml.toml");
    fs::write(&manifest, "[workspace]\nschemas=['model.xsd']\n[codegen.python]\noutput='generated'\nslots=false\nkw_only=false\n").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["build", "--config"])
        .arg(&manifest)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(fs::read_to_string(dir.path().join("generated/model.py"))
        .unwrap()
        .contains("@dataclass(slots=False, kw_only=False)"));
    for backend in ["dataclass", "pydantic"] {
        let result = Command::new(env!("CARGO_BIN_EXE_polyxml"))
            .arg("generate")
            .arg(&schema)
            .args([
                "--lang",
                "py",
                "--backend",
                backend,
                "--feature",
                "slots",
                "--feature",
                "kw-only",
            ])
            .arg("--out")
            .arg(dir.path().join("features"))
            .output()
            .unwrap();
        assert_eq!(result.status.success(), backend == "dataclass");
        if backend == "dataclass" {
            assert!(fs::read_to_string(dir.path().join("features/model.py"))
                .unwrap()
                .contains("@dataclass(slots=True, kw_only=True)"));
        } else {
            assert!(String::from_utf8_lossy(&result.stderr)
                .contains("require the Python dataclass backend"));
        }
    }
    fs::write(
        &manifest,
        "[workspace]\nschemas=['model.xsd']\n[codegen.python]\nslots=false\nfeatures=['slots']\n",
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["build", "--config"])
        .arg(&manifest)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("conflicts"));
}

#[test]
fn completion_candidates_follow_target_validation() {
    for (kind, words, expected) in [
        (
            "backend",
            vec!["generate"],
            vec!["dataclass", "pydantic", "aot"],
        ),
        (
            "backend",
            vec!["generate", "--lang", "ts"],
            vec!["interfaces", "zod", "valibot", "typebox", "standard"],
        ),
        (
            "backend",
            vec!["generate", "--lang=java"],
            vec!["standard", "jackson"],
        ),
        (
            "backend",
            vec!["generate", "-lcs"],
            vec!["standard", "source-gen"],
        ),
        (
            "backend",
            vec!["generate", "-l", "java", "-l", "cpp"],
            vec!["standard"],
        ),
        (
            "backend",
            vec!["generate", "-l", "python", "-l", "java"],
            vec![],
        ),
        ("backend", vec!["generate", "--lang", "unknown"], vec![]),
        (
            "feature",
            vec!["generate", "--lang", "rust"],
            vec!["zero-copy", "rkyv", "phf"],
        ),
        ("feature", vec!["generate", "--backend=pydantic"], vec![]),
        (
            "style",
            vec!["generate", "--lang", "java"],
            vec!["record", "pojo", "class"],
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
            .args(["__complete", kind, "--"])
            .args(&words)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .collect::<Vec<_>>(),
            expected,
            "{kind}: {words:?}"
        );
    }
    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["__complete", "flag", "--", "generate"])
        .output()
        .unwrap();
    let flags = String::from_utf8_lossy(&output.stdout);
    assert!(flags.contains("--feature"));
}

#[test]
fn bash_completion_filters_values_in_the_shell() {
    let dir = tempdir().unwrap();
    let script = dir.path().join("polyxml.bash");
    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["completions", "bash"])
        .output()
        .unwrap();
    assert!(output.status.success());
    fs::write(&script, output.stdout).unwrap();
    for (words, expected) in [
        (
            vec!["generate", "--lang", "java", "--backend", ""],
            vec!["standard", "jackson"],
        ),
        (
            vec!["generate", "--lang", "=", "ts", "--backend", "=", "v"],
            vec!["valibot"],
        ),
        (
            vec!["generate", "--lang=ts", "--backend", "v"],
            vec!["valibot"],
        ),
        (
            vec!["generate", "--lang", "rust", "--feature", "zero-copy,r"],
            vec!["zero-copy,rkyv"],
        ),
        (
            vec!["generate", "-l", "cs", "--style=record-s"],
            vec!["--style=record-struct"],
        ),
        (
            vec!["generate", "-l", "java", "-l", "go", "--backend", ""],
            vec!["standard"],
        ),
    ] {
        let output = Command::new("bash")
            .args([
                "--noprofile",
                "--norc",
                "-c",
                r#"
source "$1"
shift
COMP_WORDS=("$@")
COMP_CWORD=$((${#COMP_WORDS[@]} - 1))
_polyxml
printf '%s\n' "${COMPREPLY[@]}"
"#,
                "completion-test",
            ])
            .arg(&script)
            .arg(env!("CARGO_BIN_EXE_polyxml"))
            .args(&words)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .collect::<Vec<_>>(),
            expected,
            "{words:?}"
        );
    }
}

#[test]
fn zsh_completion_filters_values_when_available() {
    if Command::new("zsh").arg("--version").output().is_err() {
        return;
    }
    let dir = tempdir().unwrap();
    let script = dir.path().join("_polyxml");
    let generated = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["completions", "zsh"])
        .output()
        .unwrap();
    assert!(generated.status.success());
    fs::write(&script, generated.stdout).unwrap();
    let result = Command::new("zsh")
        .args([
            "-f",
            "-c",
            r#"
fpath=("${1:h}" $fpath)
autoload -Uz _polyxml
compadd() { print -l -- "${values[@]}"; }
_arguments() { _polyxml_values backend; }
words=("$2" generate --lang java --backend '')
CURRENT=6
_polyxml
"#,
            "completion-test",
        ])
        .arg(&script)
        .arg(env!("CARGO_BIN_EXE_polyxml"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["standard", "jackson"]
    );
}

#[test]
fn fish_completion_filters_values_when_available() {
    if Command::new("fish").arg("--version").output().is_err() {
        return;
    }
    let dir = tempdir().unwrap();
    let script = dir.path().join("polyxml.fish");
    let generated = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["completions", "fish"])
        .output()
        .unwrap();
    assert!(generated.status.success());
    fs::write(&script, generated.stdout).unwrap();
    let binary_dir = std::path::Path::new(env!("CARGO_BIN_EXE_polyxml"))
        .parent()
        .unwrap();
    let result = Command::new("fish")
        .args([
            "--no-config",
            "-c",
            r#"
source $argv[1]
set -gx PATH $argv[2] $PATH
complete -C 'polyxml generate --lang java --backend '
"#,
        ])
        .arg(&script)
        .arg(binary_dir)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout)
            .lines()
            .collect::<Vec<_>>(),
        ["jackson", "standard"]
    );
}

#[test]
fn test_cli_python_aot_generation() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("flight.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:flight">
            <xs:element name="Flight">
                <xs:complexType>
                    <xs:sequence>
                        <xs:element name="callsign" type="xs:string"/>
                        <xs:element name="altitude" type="xs:int"/>
                    </xs:sequence>
                </xs:complexType>
            </xs:element>
        </xs:schema>"#,
    )
    .unwrap();

    let out_dir = dir.path().join("aot_flight");
    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            schema_file.to_str().unwrap(),
            "-l",
            "python",
            "-b",
            "aot",
            "-p",
            "aot_flight",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute generate");

    assert!(
        output.status.success(),
        "CLI generate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(out_dir.join("Cargo.toml").exists(), "Cargo.toml must exist");
    assert!(
        out_dir.join("pyproject.toml").exists(),
        "pyproject.toml must exist"
    );
    assert!(out_dir.join("README.md").exists(), "README.md must exist");
    assert!(out_dir.join("src/lib.rs").exists(), "src/lib.rs must exist");
    assert!(
        out_dir.join("aot_flight.pyi").exists(),
        "aot_flight.pyi stub must exist"
    );
    assert!(
        out_dir.join("py.typed").exists(),
        "py.typed marker must exist"
    );

    let lib_rs = fs::read_to_string(out_dir.join("src/lib.rs")).unwrap();
    assert!(
        lib_rs.contains("#[pymodule]"),
        "lib.rs must have #[pymodule]"
    );
    assert!(lib_rs.contains("#[pyclass"), "lib.rs must have #[pyclass]");
}

#[test]
fn test_cli_python_aot_rejects_dataclass_style_and_slots() {
    let dir = tempdir().unwrap();
    let schema_file = dir.path().join("test.xsd");
    fs::write(
        &schema_file,
        r#"<?xml version="1.0"?>
        <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test">
            <xs:element name="Root" type="xs:string"/>
        </xs:schema>"#,
    )
    .unwrap();

    // --style dataclass with --backend aot must fail
    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            schema_file.to_str().unwrap(),
            "-l",
            "python",
            "-b",
            "aot",
            "--style",
            "dataclass",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("style 'dataclass' requires the Python dataclass backend"));

    // --slots with --backend aot must fail
    let output2 = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args([
            "generate",
            schema_file.to_str().unwrap(),
            "-l",
            "python",
            "-b",
            "aot",
            "--feature",
            "slots",
        ])
        .output()
        .unwrap();
    assert!(!output2.status.success());
    assert!(String::from_utf8_lossy(&output2.stderr)
        .contains("slots and kw-only require the Python dataclass backend"));
}

#[test]
fn test_cli_shared_modules_deduplication() {
    let dir = tempdir().unwrap();
    let fixtures =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/shared_modules");
    fs::copy(fixtures.join("common.xsd"), dir.path().join("common.xsd")).unwrap();
    fs::copy(fixtures.join("auth.xsd"), dir.path().join("auth.xsd")).unwrap();
    fs::copy(fixtures.join("billing.xsd"), dir.path().join("billing.xsd")).unwrap();

    let manifest_file = dir.path().join("polyxml.toml");
    let manifest_content = r#"
[workspace]
name = "enterprise_services"
output_base_dir = "./out"
go_module = "enterprise/models"

[modules.common]
schemas = ["common.xsd"]

[modules.auth]
schemas = ["auth.xsd"]
depends_on = ["common"]

[modules.billing]
schemas = ["billing.xsd"]
depends_on = ["common"]

[[generate]]
target = "rust"
output = "rs"

[[generate]]
target = "python"
output = "py"

[[generate]]
target = "typescript"
output = "ts"

[[generate]]
target = "go"
output = "go"

[[generate]]
target = "csharp"
output = "cs"

[[generate]]
target = "java"
output = "java"
package = "com.enterprise.models"

[[generate]]
target = "cpp"
output = "cpp"
package = "enterprise::models"
"#;
    fs::write(&manifest_file, manifest_content).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_polyxml"))
        .args(["build", "--config", manifest_file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let out_dir = dir.path().join("out");

    // 1. Rust: common has Person, billing imports common::Person without duplicate definition
    let common_rs = fs::read_to_string(out_dir.join("rs/common/common.rs")).unwrap();
    assert!(common_rs.contains("pub struct Person"));
    let billing_rs = fs::read_to_string(out_dir.join("rs/billing/billing.rs")).unwrap();
    assert!(billing_rs.contains("pub struct BillingMessage"));
    assert!(!billing_rs.contains("pub struct Person {"));
    assert!(billing_rs.contains("use super::super::common::{Person};"));

    // 2. Python: common has Person, billing imports Person from common without redefining
    let common_py = fs::read_to_string(out_dir.join("py/common/common.py")).unwrap();
    assert!(common_py.contains("class Person"));
    let billing_py = fs::read_to_string(out_dir.join("py/billing/billing.py")).unwrap();
    assert!(billing_py.contains("class BillingMessage"));
    assert!(!billing_py.contains("class Person("));
    assert!(billing_py.contains("from ..common.common import Person as Person"));

    // 3. TypeScript: common has Person, billing imports Person from common
    let common_ts = fs::read_to_string(out_dir.join("ts/common/common.ts")).unwrap();
    assert!(common_ts.contains("export interface Person"));
    let billing_ts = fs::read_to_string(out_dir.join("ts/billing/billing.ts")).unwrap();
    assert!(billing_ts.contains("export interface BillingMessage"));
    assert!(!billing_ts.contains("export interface Person {"));
    assert!(billing_ts.contains("import { type Person } from \"../common/common\";"));

    // 4. Go: common has Person, billing has BillingMessage with common.Person alias
    let common_go = fs::read_to_string(out_dir.join("go/common/common.go")).unwrap();
    assert!(common_go.contains("type Person struct"));
    let billing_go = fs::read_to_string(out_dir.join("go/billing/billing.go")).unwrap();
    assert!(billing_go.contains("type BillingMessage struct"));
    assert!(billing_go.contains("common \"enterprise/models/common\""));
    assert!(billing_go.contains("type Person = common.Person"));

    // 5. C#: common has Person, billing imports common namespace
    let common_cs = fs::read_to_string(out_dir.join("cs/common/Common.cs")).unwrap();
    assert!(common_cs.contains("public record Person"));
    let billing_cs = fs::read_to_string(out_dir.join("cs/billing/Billing.cs")).unwrap();
    assert!(billing_cs.contains("public record BillingMessage"));
    assert!(!billing_cs.contains("public record Person("));
    assert!(billing_cs.contains("using Generated.Common;"));

    // 6. Java: common has Person.java, billing has BillingMessage.java importing common.Person
    assert!(out_dir.join("java/common/Person.java").exists());
    let billing_java =
        fs::read_to_string(out_dir.join("java/billing/BillingMessage.java")).unwrap();
    assert!(billing_java.contains("import com.enterprise.models.common.Person;"));
    let auth_java = fs::read_to_string(out_dir.join("java/auth/AuthMessage.java")).unwrap();
    assert!(auth_java.contains("com.enterprise.models.common.Person shared"));
    assert!(auth_java.contains("Person local"));

    // 7. C++: common has Person, billing includes common.hpp and aliases using enterprise::models::common::Person
    let common_cpp = fs::read_to_string(out_dir.join("cpp/common/common.hpp")).unwrap();
    assert!(common_cpp.contains("struct Person"));
    let billing_cpp = fs::read_to_string(out_dir.join("cpp/billing/billing.hpp")).unwrap();
    assert!(billing_cpp.contains("struct BillingMessage"));
    assert!(!billing_cpp.contains("struct Person {"));
    assert!(billing_cpp.contains("#include \"../common/common.hpp\""));
    assert!(billing_cpp.contains("using enterprise::models::common::Person;"));

    // 8. Collision disambiguation in auth: local Person kept as Person, imported common:Person aliased to Person2
    let auth_rs = fs::read_to_string(out_dir.join("rs/auth/auth.rs")).unwrap();
    assert!(auth_rs.contains("pub struct Person<'a>"));
    assert!(auth_rs.contains("pub struct AuthMessage<'a>"));
    assert!(auth_rs.contains("pub shared: Person2<'a>"));
    assert!(auth_rs.contains("pub local: Person<'a>"));
    assert!(auth_rs.contains("use super::super::common::{Person as Person2};"));

    let auth_py = fs::read_to_string(out_dir.join("py/auth/auth.py")).unwrap();
    assert!(auth_py.contains("class Person:"));
    assert!(auth_py.contains("class AuthMessage:"));
    assert!(auth_py.contains("from ..common.common import Person as Person2"));

    let auth_ts = fs::read_to_string(out_dir.join("ts/auth/auth.ts")).unwrap();
    assert!(auth_ts.contains("export interface Person {"));
    assert!(auth_ts.contains("export interface AuthMessage {"));
    assert!(auth_ts.contains("import { type Person as Person2 } from \"../common/common\";"));

    let auth_go = fs::read_to_string(out_dir.join("go/auth/auth.go")).unwrap();
    assert!(auth_go.contains("type Person struct"));
    assert!(auth_go.contains("type AuthMessage struct"));
    assert!(auth_go.contains("type Person2 = common.Person"));

    let auth_cs = fs::read_to_string(out_dir.join("cs/auth/Auth.cs")).unwrap();
    assert!(auth_cs.contains("public record Person"));
    assert!(auth_cs.contains("public record AuthMessage"));
    assert!(auth_cs.contains("using Person2 = Generated.Common.Person;"));

    let auth_cpp = fs::read_to_string(out_dir.join("cpp/auth/auth.hpp")).unwrap();
    assert!(auth_cpp.contains("struct Person"));
    assert!(auth_cpp.contains("struct AuthMessage"));
    assert!(auth_cpp.contains("using Person2 = enterprise::models::common::Person;"));

    if Command::new("go").arg("version").output().is_ok() {
        let build = Command::new("go")
            .args(["build", "./..."])
            .current_dir(out_dir.join("go"))
            .output()
            .unwrap();
        assert!(
            build.status.success(),
            "{}",
            String::from_utf8_lossy(&build.stderr)
        );
    }
    if Command::new("javac").arg("-version").output().is_ok() {
        let sources = [
            "common/Person.java",
            "auth/Person.java",
            "auth/AuthMessage.java",
            "billing/BillingMessage.java",
        ];
        let output = Command::new("javac")
            .arg("-d")
            .arg(dir.path().join("classes"))
            .args(
                sources
                    .iter()
                    .map(|source| out_dir.join("java").join(source)),
            )
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
