use mask_parser::maskfile::Script;
use std::{
    fmt::{Debug, Display},
    io,
    path::Path,
    process::Command,
};

#[derive(Debug)]
pub enum LintResultType {
    Warning,
    Findings,
}

#[derive(Debug)]
pub struct LintResult {
    pub message: String,
    pub result_type: LintResultType,
}

impl LintResult {
    pub fn warning(message: String) -> Self {
        LintResult { message, result_type: LintResultType::Warning }
    }

    pub fn findings(message: String) -> Self {
        LintResult { message, result_type: LintResultType::Findings }
    }
}

pub trait LanguageHandler: Display {
    fn file_extension(&self) -> &'static str {
        ""
    }
    fn content(&self, script: &Script) -> Result<String, io::Error> {
        Ok(script.source.clone())
    }
    fn execute(&self, path: &Path) -> Result<LintResult, io::Error>;
}

#[derive(Debug)]
pub struct Catchall;
impl Display for Catchall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "catchall")
    }
}
impl LanguageHandler for Catchall {
    fn execute(&self, _: &Path) -> Result<LintResult, io::Error> {
        Ok(LintResult::warning("no linter found for target".to_string()))
    }
}

#[derive(Debug)]
pub struct Shellcheck;
impl Display for Shellcheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "shellcheck")
    }
}

impl LanguageHandler for Shellcheck {
    fn file_extension(&self) -> &'static str {
        ".sh"
    }
    fn execute(&self, path: &Path) -> Result<LintResult, io::Error> {
        let output = Command::new("shellcheck").arg(path).output()?;
        let findings = String::from_utf8_lossy(&output.stdout)
            .trim()
            .replace(&format!("{} ", path.to_string_lossy()), "");
        Ok(LintResult::findings(findings))
    }
    fn content(&self, script: &Script) -> Result<String, io::Error> {
        let mut res = format!("#!/bin/usr/env {}\n", script.executor);
        res.push_str(&script.source);
        Ok(res)
    }
}

pub struct Ruff;
impl Display for Ruff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ruff")
    }
}

impl LanguageHandler for Ruff {
    fn file_extension(&self) -> &'static str {
        ".py"
    }
    fn execute(&self, path: &Path) -> Result<LintResult, io::Error> {
        let output = Command::new("ruff")
            .arg("check")
            .arg("--output-format=full") // show context in source
            .arg("--no-cache")
            .arg("--quiet") // don't print anything on success
            .arg(path)
            .output()?;
        let mut valid_lines: Vec<String> = vec![];
        for line in String::from_utf8_lossy(&output.stdout).trim().lines() {
            // breaks on "Found x error."
            if line.starts_with("Found ") {
                break;
            }

            valid_lines.push(line.replace(&format!("{}:", path.to_string_lossy()), "line "));
        }
        Ok(LintResult::findings(valid_lines.join("\n").trim().to_string()))
    }
}

pub struct Rubocop;
impl Display for Rubocop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rubocop")
    }
}

impl LanguageHandler for Rubocop {
    fn file_extension(&self) -> &'static str {
        ".rb"
    }
    fn execute(&self, path: &Path) -> Result<LintResult, io::Error> {
        let output = Command::new("rubocop")
            .arg("--format=clang")
            .arg("--display-style-guide")
            .arg(path)
            .output()?;
        let findings = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|l| !l.contains("1 file inspected"))
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .replace(&format!("{}:", path.to_string_lossy()), "line ");
        Ok(LintResult::findings(findings))
    }
}
