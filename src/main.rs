use anyhow::anyhow;
use clap::{command, Parser, Subcommand};
use owo_colors::OwoColorize;
use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};

mod handlers;
use handlers::{Catchall, LanguageHandler, LintResultType, Rubocop, Ruff, Shellcheck};

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
