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
    if combined.is_empty() { return Ok("(exit 0, no output)".into()); }
    // Strip HTML tags for curl URL fetches so model sees readable text
    let is_html_fetch = (cmd.contains("curl") || cmd.contains("wget"))
        && combined.trim_start().starts_with('<');
    if is_html_fetch {
        let text = strip_html(&combined);
        return Ok(if text.is_empty() { "(no readable content)".into() } else { text });
    }
    Ok(combined)
}

fn strip_html(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    let mut _in_script = false;
    let mut buf = String::new();
    let lower = html.to_lowercase();
    let bytes = html.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // skip <script>...</script> and <style>...</style>
        if !in_tag && i + 7 < bytes.len() {
            let chunk = &lower[i..std::cmp::min(i+8, lower.len())];
            if chunk.starts_with("<script") || chunk.starts_with("<style") {
                let end_tag = if chunk.starts_with("<script") { "</script>" } else { "</style>" };
                if let Some(end) = lower[i..].find(end_tag) {
                    i += end + end_tag.len();
                    continue;
                }
            }
        }
        let c = bytes[i] as char;
        if c == '<' { in_tag = true; buf.clear(); }
        else if c == '>' { in_tag = false; }
        else if !in_tag {
            out.push(c);
        }
        i += 1;
    }
    // collapse whitespace and deduplicate blank lines
    let mut result = String::new();
    let mut blank = 0u32;
    for line in out.lines() {
        let t = line.trim();
        if t.is_empty() { blank += 1; if blank <= 1 { result.push('\n'); } }
        else { blank = 0; result.push_str(t); result.push('\n'); }
    }
    result.trim().to_string()
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

    // 1. Try git apply
    let out = Command::new("git")
        .args(["apply", "--whitespace=fix", patch_path.to_str().unwrap()])
        .current_dir(cwd)
        .output()?;

    if out.status.success() {
        let _ = fs::remove_file(&patch_path);
        return Ok("(applied ok)".into());
    }

    let git_err = format!("{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ).trim().to_string();

    // 2. Try patch -p1
    let out_p1 = Command::new("patch")
        .args(["-p1", "-i", patch_path.to_str().unwrap()])
        .current_dir(cwd)
        .output();

    if let Ok(o) = out_p1 {
        if o.status.success() {
             let _ = fs::remove_file(&patch_path);
             return Ok("(applied with patch -p1)".into());
        }
    }

    let _ = fs::remove_file(&patch_path);
    Ok(format!("patch failed: {}", git_err))
}
