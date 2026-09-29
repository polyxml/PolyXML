use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use glob::glob;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error reading configuration: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML syntax error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Invalid glob pattern '{pattern}': {error}")]
    GlobPattern {
        pattern: String,
        error: glob::PatternError,
    },

    #[error("Failed to read glob path: {0}")]
    Glob(#[from] glob::GlobError),

    #[error("Module '{module}' depends on unknown module '{dependency}'")]
    UnknownModuleDependency { module: String, dependency: String },

    #[error("Cycle in workspace module dependencies involving '{0}'")]
    ModuleCycle(String),

    #[error("Invalid module name '{0}': use ASCII letters, digits, and underscores, starting with a letter or underscore")]
    InvalidModuleName(String),
}

/// The top-level `polyxml.toml` workspace manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceManifest {
    pub workspace: Option<WorkspaceSection>,
    #[serde(default)]
    pub generate: Vec<TargetConfig>,
    pub codegen: Option<HashMap<String, CodegenTargetConfig>>,
    #[serde(default)]
    pub modules: BTreeMap<String, ModuleConfig>,
}

/// A schema package compiled once and imported by dependent packages.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleConfig {
    #[serde(default)]
    pub schemas: Vec<String>,
    #[serde(default)]
    pub root_elements: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSection {
    pub name: Option<String>,
    #[serde(default)]
    pub schemas: Vec<String>,
    #[serde(default)]
    pub root_elements: Vec<String>,
    pub include_dirs: Option<Vec<String>>,
    pub output_base_dir: Option<String>,
    pub custom_header: Option<String>,
    pub go_module: Option<String>,
}

/// Target configuration from either `[[generate]]` or `[codegen.<target>]`.
/// Unknown keys are rejected so misspelled or unsupported options fail
/// instead of being silently ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetConfig {
    pub target: String,
    pub output: String,
    pub enabled: Option<bool>,
    pub backend: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
    pub package: Option<String>,
    pub namespace: Option<String>,
    pub strict_facets: Option<bool>,
    pub slots: Option<bool>,
    pub kw_only: Option<bool>,
    pub zero_copy: Option<bool>,
    pub codecs: Option<bool>,
    pub standard: Option<String>,
    pub derive_traits: Option<Vec<String>>,
    pub box_cycles: Option<bool>,
    pub modules: Option<bool>,
    pub mode: Option<String>,
    pub serializer: Option<String>,
    pub style: Option<String>,
    pub custom_header: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodegenTargetConfig {
    pub enabled: Option<bool>,
    pub output: Option<String>,
    pub backend: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
    pub package: Option<String>,
    pub namespace: Option<String>,
    pub strict_facets: Option<bool>,
    pub slots: Option<bool>,
    pub kw_only: Option<bool>,
    pub zero_copy: Option<bool>,
    pub codecs: Option<bool>,
    pub standard: Option<String>,
    pub derive_traits: Option<Vec<String>>,
    pub box_cycles: Option<bool>,
    pub modules: Option<bool>,
    pub mode: Option<String>,
    pub serializer: Option<String>,
    pub style: Option<String>,
    pub custom_header: Option<String>,
}

impl std::str::FromStr for WorkspaceManifest {
    type Err = ConfigError;

    fn from_str(toml_str: &str) -> Result<Self, Self::Err> {
        let manifest: WorkspaceManifest = toml::from_str(toml_str)?;
        Ok(manifest)
    }
}

impl WorkspaceManifest {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        content.parse()
    }

    /// Retrieve all configured target configurations, combining `[[generate]]`
    /// and `[codegen.<target>]` definitions.
    pub fn resolved_targets(&self) -> Vec<TargetConfig> {
        let mut targets = Vec::new();

        let ws_header = self
            .workspace
            .as_ref()
            .and_then(|w| w.custom_header.clone());

        // 1. Array of tables [[generate]]
        for gen in &self.generate {
            if gen.enabled.unwrap_or(true) {
                let mut target = gen.clone();
                if target.custom_header.is_none() {
                    target.custom_header = ws_header.clone();
                }
                targets.push(target);
            }
        }

        // 2. Table-based [codegen.<lang>]
        if let Some(ref codegen_map) = self.codegen {
            for (lang, cfg) in codegen_map {
                if cfg.enabled.unwrap_or(true) {
                    let output = cfg
                        .output
                        .clone()
                        .unwrap_or_else(|| format!("generated/{}", lang));

                    targets.push(TargetConfig {
                        target: lang.clone(),
                        output,
                        enabled: cfg.enabled,
                        backend: cfg.backend.clone(),
                        features: cfg.features.clone(),
                        package: cfg.package.clone(),
                        namespace: cfg.namespace.clone(),
                        strict_facets: cfg.strict_facets,
                        slots: cfg.slots,
                        kw_only: cfg.kw_only,
                        zero_copy: cfg.zero_copy,
                        codecs: cfg.codecs,
                        standard: cfg.standard.clone(),
                        derive_traits: cfg.derive_traits.clone(),
                        box_cycles: cfg.box_cycles,
                        modules: cfg.modules,
                        mode: cfg.mode.clone(),
                        serializer: cfg.serializer.clone(),
                        style: cfg.style.clone(),
                        custom_header: cfg.custom_header.clone().or_else(|| ws_header.clone()),
                    });
                }
            }
        }

        targets
    }

    /// Expand all schema glob patterns in `workspace.schemas` relative to base directory.
    pub fn expand_schemas(&self, base_dir: &Path) -> Result<Vec<PathBuf>, ConfigError> {
        let Some(ref ws) = self.workspace else {
            return Ok(Vec::new());
        };

        expand_patterns(&ws.schemas, base_dir)
    }

    pub fn expand_module_schemas(
        &self,
        name: &str,
        base_dir: &Path,
    ) -> Result<Vec<PathBuf>, ConfigError> {
        let Some(module) = self.modules.get(name) else {
            return Ok(Vec::new());
        };
        expand_patterns(&module.schemas, base_dir)
    }

    pub fn module_order(&self) -> Result<Vec<String>, ConfigError> {
        for name in self.modules.keys() {
            let mut chars = name.chars();
            if !chars
                .next()
                .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
                || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            {
                return Err(ConfigError::InvalidModuleName(name.clone()));
            }
        }
        let mut order = Vec::new();
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        fn visit(
            name: &str,
            manifest: &WorkspaceManifest,
            visiting: &mut HashSet<String>,
            visited: &mut HashSet<String>,
            order: &mut Vec<String>,
        ) -> Result<(), ConfigError> {
            if visited.contains(name) {
                return Ok(());
            }
            if !visiting.insert(name.to_string()) {
                return Err(ConfigError::ModuleCycle(name.to_string()));
            }
            let module = &manifest.modules[name];
            for dependency in &module.depends_on {
                if !manifest.modules.contains_key(dependency) {
                    return Err(ConfigError::UnknownModuleDependency {
                        module: name.to_string(),
                        dependency: dependency.clone(),
                    });
                }
                visit(dependency, manifest, visiting, visited, order)?;
            }
            visiting.remove(name);
            visited.insert(name.to_string());
            order.push(name.to_string());
            Ok(())
        }
        for name in self.modules.keys() {
            visit(name, self, &mut visiting, &mut visited, &mut order)?;
        }
        Ok(order)
    }
}

fn expand_patterns(patterns: &[String], base_dir: &Path) -> Result<Vec<PathBuf>, ConfigError> {
    let mut paths = Vec::new();
    for pattern in patterns {
        let full_pattern = if Path::new(pattern).is_absolute() {
            pattern.clone()
        } else {
            base_dir.join(pattern).to_string_lossy().to_string()
        };

        let entries = glob(&full_pattern).map_err(|e| ConfigError::GlobPattern {
            pattern: full_pattern.clone(),
            error: e,
        })?;

        for entry in entries {
            let path = entry?;
            if path.is_file() {
                paths.push(path);
            }
        }
    }

    paths.sort();
    paths.dedup();
    Ok(paths)
}
