use anyhow::Result;
use std::{fs, path::PathBuf, process::Command};

pub fn run_cmd(cwd: &PathBuf, cmd: &str) -> Result<String> {
    // collapse line continuations so model can pass multi-line commands
    let cmd = cmd.replace("\\\n", " ");
    let out = Command::new("sh").arg("-c").arg(&cmd).current_dir(cwd).output()?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ).trim().to_string();
    Ok(if combined.is_empty() { "(exit 0, no output)".into() } else { combined })
}

pub fn write_file(cwd: &PathBuf, path: &str, content: &str) -> Result<String> {
    let full = cwd.join(path);
    // Refuse to overwrite large existing files — use apply_patch instead
    if full.exists() {
        let existing = fs::read_to_string(&full).unwrap_or_default();
        let lines = existing.lines().count();
        if lines > 300 {
            return Ok(format!(
                "ERROR: {} has {} lines. Use apply_patch for targeted edits instead of rewriting the whole file.",
                path, lines
            ));
        }
    }
    if let Some(p) = full.parent() { fs::create_dir_all(p)?; }
    fs::write(&full, content)?;
    Ok(format!("wrote {} ({} bytes)", path, content.len()))
}

pub fn apply_patch(cwd: &PathBuf, diff: &str) -> Result<String> {
    let patch_path = cwd.join(format!(".nanocode_{}.diff",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos()).unwrap_or(0)));
    fs::write(&patch_path, diff)?;
    // git apply is more forgiving than patch -p1
    let out = Command::new("git")
        .args(["apply", "--whitespace=fix", patch_path.to_str().unwrap()])
        .current_dir(cwd)
        .output()?;
    let _ = fs::remove_file(&patch_path);
    if out.status.success() {
        return Ok("(applied ok)".into());
    }
    let err = format!("{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ).trim().to_string();
    Ok(format!("patch failed: {err}"))
}
