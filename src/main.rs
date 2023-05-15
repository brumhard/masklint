use anyhow::Result;
use clap::{command, Parser, Subcommand};
use mask_parser::maskfile::Script;
use owo_colors::OwoColorize;
use std::{
    fmt::format,
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

        // let section_header = format!("'{}'", );
        println!("{}", command.name.bold().cyan().underline());

        let linter: Box<dyn Linter> = match script.executor.as_str() {
            "sh" | "bash" | "zsh" => Box::new(Shellcheck {}),
            "py" | "python" => Box::new(Pylint {}),
            "rb" | "ruby" => Box::new(Rubocop {}),
            lang => {
                println!("no linter for language {lang} found");
                continue;
            }
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

const PYLINT_IGNORES: &'static [&str] = &[
    "C0114", //https://pylint.readthedocs.io/en/latest/user_guide/messages/convention/missing-module-docstring.html
];
struct Pylint;
impl Linter for Pylint {
    fn file_extension(&self) -> &'static str {
        ".py"
    }
    fn execute(&self, path: &Path) -> Result<String> {
        let output = Command::new("pylint").arg(path).output()?;
        let mut valid_lines: Vec<String> = vec![];
        for line in String::from_utf8_lossy(&output.stdout).trim().lines() {
            if line.starts_with("-------") {
                break;
            }
            if line.starts_with("******") {
                continue;
            }
            // if the line contains any of the ignores skip
            if PYLINT_IGNORES.iter().find(|&ignore| line.contains(ignore)).is_some() {
                continue;
            }

            valid_lines.push(line.replace(&format!("{}:", path.to_string_lossy()), "line "))
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
