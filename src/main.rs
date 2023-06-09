use anyhow::Result;
use clap::{command, Parser, Subcommand};
use mask_parser::maskfile::Script;
use owo_colors::OwoColorize;
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(global = true, long, default_value = "maskfile.md")]
    /// Path to a different maskfile you want to use
    maskfile: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs the linters.
    Run {},
    /// Extracts all the commands from the maskfile and dumps them as files
    /// into the defined directory.
    Dump {
        #[arg(short, long)]
        output: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let content = fs::read_to_string(cli.maskfile)?;
    let maskfile = mask_parser::parse(content);

    // keeping it here to not let it go out of scope
    // TODO: refactor to only create tempdir if needed
    let tmp_dir = tempfile::tempdir()?;
    let mut out_dir = tmp_dir.path().to_path_buf();
    if let Commands::Dump { output } = &cli.command {
        drop(tmp_dir);
        out_dir = output.parse()?;
        fs::create_dir_all(&out_dir)?;
    }

    for command in maskfile.commands {
        let script = match command.script {
            None => continue,
            Some(s) => s,
        };

        let linter: Box<dyn Linter> = match script.executor.as_str() {
            "sh" | "bash" | "zsh" => Box::new(Shellcheck {}),
            "py" | "python" => Box::new(Ruff {}),
            "rb" | "ruby" => Box::new(Rubocop {}),
            _ => Box::new(Catchall {}),
        };

        let mut file_name = command.name.clone();
        file_name.push_str(linter.file_extension());
        let file_path = out_dir.join(&file_name);
        let mut script_file = File::options().create_new(true).append(true).open(&file_path)?;
        let content = linter.content(&script)?;
        script_file.write_all(content.as_bytes())?;

        if matches!(cli.command, Commands::Dump { .. }) {
            continue;
        }

        let findings = linter.execute(&file_path)?;
        if findings.is_empty() {
            continue;
        }

        println!("{}", command.name.bold().cyan().underline());
        println!("{findings}\n");
    }
    Ok(())
}

trait Linter {
    fn file_extension(&self) -> &'static str {
        ""
    }
    fn content(&self, script: &Script) -> Result<String> {
        Ok(script.source.clone())
    }
    fn execute(&self, path: &Path) -> Result<String>;
}

struct Catchall;
impl Linter for Catchall {
    fn execute(&self, _: &Path) -> Result<String> {
        Ok("no linter found for target".to_string())
    }
}

struct Shellcheck;
impl Linter for Shellcheck {
    fn file_extension(&self) -> &'static str {
        ".sh"
    }
    fn execute(&self, path: &Path) -> Result<String> {
        let output = Command::new("shellcheck").arg(path).output()?;
        let findings = String::from_utf8_lossy(&output.stdout)
            .trim()
            .replace(&format!("{} ", path.to_string_lossy()), "");
        Ok(findings)
    }
    fn content(&self, script: &Script) -> Result<String> {
        let mut res = format!("#!/bin/usr/env {}\n", script.executor);
        res.push_str(&script.source);
        Ok(res)
    }
}

struct Ruff;
impl Linter for Ruff {
    fn file_extension(&self) -> &'static str {
        ".py"
    }
    fn execute(&self, path: &Path) -> Result<String> {
        let output = Command::new("ruff")
            .arg("--show-source")
            .arg("--format=text")
            .arg("--no-cache")
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
        Ok(valid_lines.join("\n").trim().to_string())
    }
}

struct Rubocop;
impl Linter for Rubocop {
    fn file_extension(&self) -> &'static str {
        ".rb"
    }
    fn execute(&self, path: &Path) -> Result<String> {
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
        Ok(findings)
    }
}
