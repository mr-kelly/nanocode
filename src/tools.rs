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
                "ERROR: {} has {} lines. Use replace for targeted edits instead of rewriting the whole file.",
                path, lines
            ));
        }
    }
    if let Some(p) = full.parent() { fs::create_dir_all(p)?; }
    fs::write(&full, content)?;
    Ok(format!("wrote {} ({} bytes)", path, content.len()))
}

pub fn replace(cwd: &PathBuf, path: &str, old: &str, new: &str) -> Result<String> {
    let full = cwd.join(path);
    if !full.exists() {
        return Ok(format!("ERROR: file {} does not exist", path));
    }
    let content = fs::read_to_string(&full)?;

    if old.is_empty() {
        return Ok("ERROR: <old> block cannot be empty".into());
    }

    if !content.contains(old) {
        // Try to find a fuzzy match by stripping leading/trailing whitespace
        let old_lines: Vec<&str> = old.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if old_lines.is_empty() {
            return Ok("ERROR: old text not found in file".into());
        }
        
        // simple fuzzy search: check if all non-empty lines of `old` appear in the file in order
        let file_lines: Vec<&str> = content.lines().collect();
        let mut match_start = None;
        for i in 0..file_lines.len() {
            let mut matched = true;
            for (j, old_line) in old_lines.iter().enumerate() {
                if i + j >= file_lines.len() || !file_lines[i + j].trim().contains(old_line) {
                    matched = false;
                    break;
                }
            }
            if matched {
                match_start = Some(i);
                break;
            }
        }

        if let Some(start) = match_start {
            let end = (start + old_lines.len() + 2).min(file_lines.len());
            let start_context = start.saturating_sub(2);
            let actual_text = file_lines[start_context..end].join("\n");
            return Ok(format!("ERROR: exact old text not found in file. However, a similar block was found. Check your indentation and line breaks, they must match exactly.\n\nHere is the actual text from the file around that location:\n```\n{}\n```\n\nPlease use this exact text in your <old> block.", actual_text));
        }

        return Ok("ERROR: exact old text not found in file. Check your indentation and line breaks, they must match exactly.".into());
    }

    let count = content.matches(old).count();
    if count > 1 {
        return Ok("ERROR: <old> block is not unique. It appears multiple times in the file. Add more context lines to make it unique.".into());
    }

    let new_content = content.replacen(old, new, 1);
    if content == new_content {
        return Ok(format!("ERROR: Replacement resulted in no changes to {}. The <new> block is identical to the <old> block.", path));
    }
    fs::write(&full, new_content)?;
    Ok(format!("replaced exact match in {}", path))
}
pub fn read_file(cwd: &PathBuf, path: &str) -> Result<String> {
    let full = cwd.join(path);
    if !full.exists() {
        return Ok(format!("ERROR: file {} does not exist", path));
    }
    let content = fs::read_to_string(&full)?;
    Ok(content)
}
