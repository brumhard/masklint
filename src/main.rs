use anyhow::anyhow;
use clap::{command, Parser, Subcommand};
use mask_parser::maskfile::Script;
use owo_colors::OwoColorize;
use std::{
    fmt::{Debug, Display},
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let content = fs::read_to_string(cli.maskfile)?;
    let maskfile = mask_parser::parse(content);

    // keeping the _tmp dir here to not let it go out of scope
    let (out_dir, _tmp) = match &cli.command {
        Commands::Dump { output } => {
            let dir: PathBuf = output.parse()?;
            fs::create_dir_all(&dir)?;
            (dir, None)
        }
        _ => {
            let tmp_dir = tempfile::tempdir()?;
            (tmp_dir.path().to_path_buf(), Some(tmp_dir))
        }
    };

    for command in maskfile.commands {
        let Some(script)  = command.script else { continue };

        let language_handler: &dyn LanguageHandler = match script.executor.as_str() {
            "sh" | "bash" | "zsh" => &Shellcheck {},
            "py" | "python" => &Ruff {},
            "rb" | "ruby" => &Rubocop {},
            _ => &Catchall {},
        };

        let mut file_name = command.name.clone();
        file_name.push_str(language_handler.file_extension());
        let file_path = out_dir.join(&file_name);
        let mut script_file = File::options().create_new(true).append(true).open(&file_path)?;
        let content = language_handler.content(&script)?;
        script_file.write_all(content.as_bytes())?;

        if matches!(cli.command, Commands::Dump { .. }) {
            continue;
        }

        let findings = language_handler.execute(&file_path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => {
                anyhow!("executable for {language_handler} not found in $PATH")
            }
            _ => anyhow!(e),
        })?;
        if findings.is_empty() {
            continue;
        }

        println!("{}", command.name.bold().cyan().underline());
        println!("{findings}\n");
    }
    Ok(())
}

trait LanguageHandler: Display {
    fn file_extension(&self) -> &'static str {
        ""
    }
    fn content(&self, script: &Script) -> Result<String, io::Error> {
        Ok(script.source.clone())
    }
    fn execute(&self, path: &Path) -> Result<String, io::Error>;
}

#[derive(Debug)]
struct Catchall;
impl Display for Catchall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "catchall")
    }
}
impl LanguageHandler for Catchall {
    fn execute(&self, _: &Path) -> Result<String, io::Error> {
        Ok("no linter found for target".to_string())
    }
}

#[derive(Debug)]
struct Shellcheck;
impl Display for Shellcheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "shellcheck")
    }
}

impl LanguageHandler for Shellcheck {
    fn file_extension(&self) -> &'static str {
        ".sh"
    }
    fn execute(&self, path: &Path) -> Result<String, io::Error> {
        let output = Command::new("shellcheck").arg(path).output()?;
        let findings = String::from_utf8_lossy(&output.stdout)
            .trim()
            .replace(&format!("{} ", path.to_string_lossy()), "");
        Ok(findings)
    }
    fn content(&self, script: &Script) -> Result<String, io::Error> {
        let mut res = format!("#!/bin/usr/env {}\n", script.executor);
        res.push_str(&script.source);
        Ok(res)
    }
}

struct Ruff;
impl Display for Ruff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ruff")
    }
}

impl LanguageHandler for Ruff {
    fn file_extension(&self) -> &'static str {
        ".py"
    }
    fn execute(&self, path: &Path) -> Result<String, io::Error> {
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
impl Display for Rubocop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rubocop")
    }
}

impl LanguageHandler for Rubocop {
    fn file_extension(&self) -> &'static str {
        ".rb"
    }
    fn execute(&self, path: &Path) -> Result<String, io::Error> {
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
