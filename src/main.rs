use anyhow::Result;
use mask_parser::maskfile::Script;
use std::{
    fmt::format,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

fn main() -> Result<()> {
    let content = fs::read_to_string("maskfile.md")?;
    let maskfile = mask_parser::parse(content);
    // let tmp_dir = tempfile::tempdir()?;
    let tmp_dir: PathBuf = "./test".parse()?;
    fs::remove_dir_all(&tmp_dir)?;
    fs::create_dir_all(&tmp_dir)?;

    for command in maskfile.commands {
        let script = match command.script {
            None => continue,
            Some(s) => s,
        };

        let linter: Box<dyn Linter> = match script.executor.as_str() {
            "sh" | "bash" | "zsh" => Box::new(Shellcheck {}),
            _ => continue,
        };

        let mut file_name = command.name.clone();
        file_name.push_str(linter.file_extension());
        let file_path = tmp_dir.join(&file_name);
        let mut script_file = File::options().create_new(true).append(true).open(&file_path)?;
        let content = linter.content(&script)?;
        script_file.write(content.as_bytes())?;

        println!("checking {}", &command.name);
        let findings = linter.execute(&file_path)?;
        println!("{}", findings);
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
        let output = Command::new("shellcheck").arg("--color=always").arg(path).output()?;
        let findings = String::from_utf8_lossy(&output.stdout)
            .replace(&format!("{} ", path.to_string_lossy()), "");
        Ok(findings)
    }
    fn content(&self, script: &Script) -> Result<String> {
        let mut res = format!("# shellcheck shell={}\n", script.executor);
        res.push_str(&script.source);
        Ok(res)
    }
}
