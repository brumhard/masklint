use anyhow::Result;
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
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

        let linter = match script.executor.as_str() {
            "sh" | "bash" => "shellcheck",
            _ => continue,
        };

        let file_path = tmp_dir.join(&command.name);
        let mut script_file = File::options().create_new(true).append(true).open(&file_path)?;
        script_file.write(format!("# shellcheck shell={}\n", script.executor).as_bytes())?;
        script_file.write(script.source.as_bytes())?;

        let path = file_path.to_string_lossy().to_string();
        println!("checking {}", &command.name);
        let output = Command::new(linter).arg("--color=always").arg(&path).output()?;
        let findings = String::from_utf8_lossy(&output.stdout).replace(&format!("{} ", path), "");
        println!("{}", findings);
    }
    Ok(())
}
