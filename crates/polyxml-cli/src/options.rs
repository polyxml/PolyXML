//! Shared validation and normalization for direct generation and both manifest forms.
use super::*;
use std::io::{Error, ErrorKind, Result};

fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidInput, message.into())
}

pub(crate) fn target_options(target: &config::TargetConfig) -> TargetEmitOptions<'_> {
    TargetEmitOptions {
        backend: target.backend.as_deref(),
        features: &target.features,
        slots: target.slots,
        kw_only: target.kw_only,
        package: target.package.as_deref().or(target.namespace.as_deref()),
        mode: target
            .mode
            .as_deref()
            .or_else(|| target.modules.filter(|v| *v).map(|_| "modules")),
        zero_copy: target.zero_copy,
        codecs: target.codecs,
        style: target.style.as_deref(),
        builder: None,
        codec: None,
        rkyv: None,
        phf: None,
        validation: None,
        custom_header: target.custom_header.as_deref(),
        split_units: target.split_units,
        chunk_size: target.chunk_size,
    }
}

impl<'a> TargetEmitOptions<'a> {
    pub(crate) fn resolve(mut self, lang: &str) -> Result<Self> {
        let language = lang.to_lowercase();
        let target = match language.as_str() {
            "python" | "py" => "python",
            "rust" | "rs" => "rust",
            "typescript" | "ts" => "typescript",
            "c++" | "cpp" => "cpp",
            "cs" | "c#" | "csharp" => "csharp",
            "java" => "java",
            "go" => "go",
            _ => return Err(invalid(format!("Unknown target '{lang}'. Supported targets: python, rust, cpp, java, typescript, go, csharp."))),
        };
        let supported_backends = match target {
            "python" => "dataclass, pydantic, aot",
            "typescript" => "interfaces, zod, valibot, typebox",
            "java" => "standard, jackson, jackson3",
            "csharp" => "standard, source-gen",
            "cpp" => "standard, glaze",
            "go" => "standard, easyjson, sonic",
            _ => "standard",
        };
        if let Some(backend) = self.backend {
            let valid = match target {
                "python" => PythonBackend::from_str_loose(backend).is_some(),
                "typescript" => TypeScriptBackend::from_str_loose(backend).is_some(),
                "java" => JavaBackend::from_str_loose(backend).is_some(),
                "cpp" => CppBackend::from_str_loose(backend).is_some(),
                "go" => GoBackend::from_str_loose(backend).is_some(),
                "csharp" => matches!(backend, "standard" | "source-gen"),
                _ => backend == "standard",
            };
            if !valid {
                return Err(invalid(format!("backend '{backend}' is not supported for target '{target}'. Supported backends: {supported_backends}.")));
            }
        }

        let supported_styles: &[&str] = match target {
            "java" => &["record", "pojo", "class"],
            "csharp" => &["record", "pojo", "class", "record-class", "record-struct"],
            "python" => &["dataclass"],
            _ => &[],
        };
        if let Some(style) = self.style {
            if !supported_styles.contains(&style) {
                return Err(invalid(format!(
                    "style '{style}' is not supported for target '{target}'. Supported styles: {}.",
                    if supported_styles.is_empty() {
                        "none".into()
                    } else {
                        supported_styles.join(", ")
                    }
                )));
            }
            if target == "python"
                && self.backend.is_some_and(|v| {
                    matches!(
                        PythonBackend::from_str_loose(v),
                        Some(PythonBackend::Pydantic | PythonBackend::Aot)
                    )
                })
            {
                return Err(invalid(
                    "style 'dataclass' requires the Python dataclass backend.",
                ));
            }
        }
        if let Some(mode) = self.mode {
            if target != "cpp" || CppMode::from_str_loose(mode).is_none() {
                return Err(invalid(format!("mode '{mode}' is not supported for target '{target}'; C++ supports header or modules.")));
            }
        }
        if target != "python" && (self.slots.is_some() || self.kw_only.is_some()) {
            return Err(invalid("slots and kw_only are Python-only options."));
        }
        let supported_features: &[&str] = match target {
            "rust" => &["zero-copy", "rkyv", "phf", "split-units"],
            "java" => &["builder", "direct-codec", "validation"],
            "python" => &["slots", "kw-only"],
            _ => &[],
        };
        for feature in self.features {
            if !supported_features.contains(&feature.as_str()) {
                return Err(invalid(format!("feature '{feature}' is not supported for target '{target}'. Supported features: {}.", if supported_features.is_empty() { "none".into() } else { supported_features.join(", ") })));
            }
            let option = match feature.as_str() {
                "zero-copy" => &mut self.zero_copy,
                "rkyv" => &mut self.rkyv,
                "phf" => &mut self.phf,
                "split-units" => &mut self.split_units,
                "builder" => &mut self.builder,
                "validation" => &mut self.validation,
                "slots" => &mut self.slots,
                "kw-only" => &mut self.kw_only,
                "direct-codec" => {
                    self.codec = Some("direct");
                    continue;
                }
                _ => unreachable!(),
            };
            if *option == Some(false) {
                return Err(invalid(format!(
                    "feature '{feature}' conflicts with an explicitly disabled option."
                )));
            }
            *option = Some(true);
        }
        if target == "python"
            && self.backend.is_some_and(|v| {
                matches!(
                    PythonBackend::from_str_loose(v),
                    Some(PythonBackend::Pydantic | PythonBackend::Aot)
                )
            })
            && (self.slots.is_some() || self.kw_only.is_some())
        {
            return Err(invalid(
                "slots and kw-only require the Python dataclass backend.",
            ));
        }
        Ok(self)
    }
}
