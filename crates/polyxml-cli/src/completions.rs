//! Shell adapters use the validator itself to keep suggestions target-aware.
use super::{Cli, TargetEmitOptions};
use clap::{CommandFactory, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

pub fn print_script(shell: Shell) {
    print!(
        "{}",
        match shell {
            Shell::Bash => include_str!("completions/bash"),
            Shell::Zsh => include_str!("completions/zsh"),
            Shell::Fish => include_str!("completions/fish"),
        }
    );
}

/// Tokens exclude the executable and the current incomplete word. Ignore
/// unrelated options, but retain every selected language and backend.
pub fn candidates(kind: &str, words: &[String]) -> Vec<String> {
    let mut languages = Vec::new();
    let mut backend = None;
    let mut iter = words.iter();
    while let Some(word) = iter.next() {
        match word.as_str() {
            "--" => break,
            "--lang" | "-l" => {
                if let Some(value) = iter.next() {
                    let value = if value == "=" {
                        iter.next()
                    } else {
                        Some(value)
                    };
                    if let Some(value) = value {
                        languages.push(value.as_str());
                    }
                }
            }
            "--backend" | "-b" => {
                backend = iter
                    .next()
                    .and_then(|value| {
                        if value == "=" {
                            iter.next()
                        } else {
                            Some(value)
                        }
                    })
                    .map(String::as_str);
            }
            _ => {
                if let Some(value) = word
                    .strip_prefix("--lang=")
                    .or_else(|| word.strip_prefix("-l").filter(|v| !v.is_empty()))
                {
                    languages.push(value);
                } else if let Some(value) = word
                    .strip_prefix("--backend=")
                    .or_else(|| word.strip_prefix("-b").filter(|v| !v.is_empty()))
                {
                    backend = Some(value);
                }
            }
        }
    }
    if languages.is_empty() {
        languages.push("python");
    }
    let choices: &[&str] = match kind {
        "lang" => &[
            "python",
            "rust",
            "cpp",
            "java",
            "typescript",
            "go",
            "csharp",
        ],
        "backend" => &[
            "dataclass",
            "pydantic",
            "aot",
            "interfaces",
            "zod",
            "valibot",
            "typebox",
            "standard",
            "jackson",
            "source-gen",
            "glaze",
            "easyjson",
            "sonic",
        ],
        "style" => &[
            "dataclass",
            "record",
            "pojo",
            "class",
            "record-class",
            "record-struct",
        ],
        "feature" => &[
            "zero-copy",
            "rkyv",
            "phf",
            "builder",
            "direct-codec",
            "slots",
            "kw-only",
        ],
        "command" => {
            return Cli::command()
                .get_subcommands()
                .filter(|c| !c.is_hide_set())
                .map(|c| c.get_name().to_string())
                .collect()
        }
        "flag" => {
            let command = Cli::command();
            let Some(subcommand) = words.first().and_then(|word| command.find_subcommand(word))
            else {
                return vec!["--help".into(), "--version".into()];
            };
            return subcommand
                .get_arguments()
                .filter(|a| !a.is_hide_set())
                .filter_map(|a| a.get_long().map(|name| format!("--{name}")))
                .chain(["--help".into(), "--version".into()])
                .collect();
        }
        _ => &[],
    };
    choices
        .iter()
        .filter(|value| {
            if kind == "lang" {
                return true;
            }
            let features = vec![value.to_string()];
            let opts = TargetEmitOptions {
                backend: if kind == "backend" {
                    Some(**value)
                } else {
                    backend
                },
                style: (kind == "style").then_some(**value),
                features: if kind == "feature" { &features } else { &[] },
                ..Default::default()
            };
            languages.iter().all(|lang| opts.resolve(lang).is_ok())
        })
        .map(|value| value.to_string())
        .collect()
}
