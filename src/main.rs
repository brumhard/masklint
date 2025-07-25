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

    #[arg(global = true, long)]
    /// Suppress warning messages
    no_warnings: bool,

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
    let context = &ProcessCommandContext {
        out_dir,
        is_dump: matches!(cli.command, Commands::Dump { .. }),
        no_warnings: cli.no_warnings,
    };

    let mut total_findings = 0;
    for command in maskfile.commands {
        total_findings += process_command(context, command, None)?;
    }

    if total_findings > 0 {
        let plural = if total_findings == 1 { "" } else { "s" };
        let error_msg = format!("{} file{} with lint failures.", total_findings, plural);
        return Err(anyhow::anyhow!(error_msg.bold().red().to_string()));
    }
    Ok(())
}

struct ProcessCommandContext {
    out_dir: PathBuf,
    is_dump: bool,
    no_warnings: bool,
}

// Function to process a command and its subcommands
fn process_command(
    context: &ProcessCommandContext,
    command: mask_parser::maskfile::Command,
    parent_name: Option<&str>,
) -> anyhow::Result<u32> {
    // Build full command name including parent
    let full_command_name = match parent_name {
        Some(parent) => format!("{} {}", parent, command.name),
        None => command.name,
    };

    let mut findings_count = 0;

    if let Some(script) = command.script {
        let language_handler: &dyn LanguageHandler = match script.executor.as_str() {
            "sh" | "bash" => &Shellcheck {},
            "py" | "python" => &Ruff {},
            "rb" | "ruby" => &Rubocop {},
            _ => &Catchall {},
        };

        let mut file_name = full_command_name.replace(" ", "_");
        file_name.push_str(language_handler.file_extension());
        let file_path = context.out_dir.join(&file_name);
        let mut script_file = File::options().create_new(true).append(true).open(&file_path)?;
        let content = language_handler.content(&script)?;
        script_file.write_all(content.as_bytes())?;

        if !context.is_dump {
            let lint_result = language_handler.execute(&file_path).map_err(|e| match e.kind() {
                io::ErrorKind::NotFound => {
                    anyhow!("executable for {language_handler} not found in $PATH")
                }
                _ => anyhow!(e),
            })?;
            if !lint_result.message.is_empty() {
                let print_results = || {
                    println!("{}", full_command_name.bold().cyan().underline());
                    println!("{}", lint_result.message);
                };
                match lint_result.result_type {
                    LintResultType::Findings => {
                        findings_count += 1;
                        print_results();
                    }
                    LintResultType::Warning => {
                        if !context.no_warnings {
                            print_results();
                        }
                    }
                }
            }
        }
    }

    // Process subcommands recursively
    if !command.subcommands.is_empty() {
        for subcmd in command.subcommands {
            findings_count += process_command(context, subcmd, Some(&full_command_name))?;
        }
    }
    Ok(findings_count)
}

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

trait LanguageHandler: Display {
    fn file_extension(&self) -> &'static str {
        ""
    }
    fn content(&self, script: &Script) -> Result<String, io::Error> {
        Ok(script.source.clone())
    }
    fn execute(&self, path: &Path) -> Result<LintResult, io::Error>;
}

#[derive(Debug)]
struct Catchall;
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
