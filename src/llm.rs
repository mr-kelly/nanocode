use anyhow::{anyhow, Result};
use genai::{
    adapter::AdapterKind,
    chat::{ChatMessage, ChatRequest, ChatStreamEvent},
    resolver::{AuthData, Endpoint},
    Client, ModelIden, ServiceTarget,
};
use std::{env, io::Write, path::PathBuf};
use crate::tools;

const DANGEROUS: &[&str] = &[
    "rm ", "rm\t", ":(){:|:&};:",
    "sudo ", "su ",
    "git push", "git reset --hard", "git clean -f",
    "dd ", "mkfs", "fdisk", "parted",
    "shutdown", "reboot", "halt",
    "curl | sh", "wget | sh", "bash <(",
];

const MAX_OUTPUT: usize = 32000;
const MAX_TURNS:  usize = 40;
const COMPRESS_AFTER: usize = 20;

const SYSTEM: &str = "\
You are Freecode — a fast, autonomous terminal agent.

TOOLS — output exactly one per turn, then wait for the result.
EVERY response must start with a <think>...</think> block explaining your reasoning and plan before calling a tool.

<run_cmd cmd=\"ls -la\" />
  Run any single-line shell command. For exploration, builds, git, etc.

<read_file path=\"path/to/file\" start=\"1\" end=\"100\" />
  Read a specific line range of a file. Use this for large files. Omit start/end to read the whole file if small.

<read_outline path=\"path/to/file\" />
  Read the AST outline (classes and methods) of a Python file with line numbers. Use this to grasp the structure of large files quickly.

<grep pattern=\"class Model\" path=\"src\" />
  Search for a string or regex pattern in the directory. Path is optional (defaults to current dir). Truncates output if >100 matches.

<find pattern=\"*.py\" />
  Find files matching a pattern.

<ls path=\"src\" />
  List files in a directory.

<write_file path=\"path/to/file\">
content line 1
content line 2
</write_file>
  Write a file with multi-line content. Always use this for file creation or complete rewrites.
  Never use shell heredocs or echo redirects for multi-line content.

<replace path=\"path/to/file\">
<old>
  exact lines to replace
</old>
<new>
  new lines
</new>
</replace>
  Replace exact lines in a file. The <old> block must match the file exactly, including leading spaces. Prefer this over write_file.

<done>summary</done>
  Signal task completion.

STRATEGY:
1. TEST-DRIVEN FIXING: Before making any file changes, you MUST write a short python script or run existing tests (via `run_cmd` and `pytest`) to reproduce the exact issue described. 
2. Verify: After patching the code, you MUST run the exact same script/test again. Do NOT call <done> until the test passes.
3. Explore first (ls, cat, git status), then act.
4. One tool per turn. Read output before next step.
5. For large files, use `<read_outline>` to see the file structure first. Then use `<read_file>` with `start` and `end` lines to inspect specific methods without exceeding your context length. Also use `rg -n 'pattern'` to find all locations.
6. Use replace for targeted edits (surgical changes). Use write_file ONLY for new files or when rewriting >50% of a file.
7. Never use shell heredocs or echo redirects for multi-line content.
8. IMPORTANT: You MUST make at least one file change (replace or write_file) before calling <done>.
9. When done: <done>concise summary</done>
10. Use 'read' (via cat/grep) to examine files before editing. You must know the exact content to patch it.
OUTPUT: one tool call only. Nothing else. No markdown.
";
const SUMMARIZATION_PROMPT: &str = "\
The messages above are a conversation to summarize. Create a structured context checkpoint summary that another LLM will use to continue the work.

Use this EXACT format:

## Goal
[What is the user trying to accomplish? Can be multiple items if the session covers different tasks.]

## Constraints & Preferences
- [Any constraints, preferences, or requirements mentioned by user]
- [Or \"(none)\" if none were mentioned]

## Progress
### Done
- [x] [Completed tasks/changes]

### In Progress
- [ ] [Current work]

### Blocked
- [Issues preventing progress, if any]

## Key Decisions
- **[Decision]**: [Brief rationale]

## Next Steps
1. [Ordered list of what should happen next]

## Critical Context
- [Any data, examples, or references needed to continue]
- [Or \"(none)\" if not applicable]

Keep each section concise. Preserve exact file paths, function names, and error messages.";

const UPDATE_SUMMARIZATION_PROMPT: &str = "\
The messages above are NEW conversation messages to incorporate into the existing summary provided in <previous-summary> tags.

Update the existing structured summary with new information. RULES:
- PRESERVE all existing information from the previous summary
- ADD new progress, decisions, and context from the new messages
- UPDATE the Progress section: move items from \"In Progress\" to \"Done\" when completed
- UPDATE \"Next Steps\" based on what was accomplished
- PRESERVE exact file paths, function names, and error messages
- If something is no longer relevant, you may remove it

Use this EXACT format:

## Goal
[Preserve existing goals, add new ones if the task expanded]

## Constraints & Preferences
- [Preserve existing, add new ones discovered]

## Progress
### Done
- [x] [Include previously done items AND newly completed items]

### In Progress
- [ ] [Current work - update based on progress]

### Blocked
- [Current blockers - remove if resolved]

## Key Decisions
- **[Decision]**: [Brief rationale] (preserve all previous, add new)

## Next Steps
1. [Update based on current state]

## Critical Context
- [Preserve important context, add new if needed]

Keep each section concise. Preserve exact file paths, function names, and error messages.";


fn openrouter_key() -> String {
    env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
        use base64::{Engine, engine::general_purpose::STANDARD};
        let b64 = "c2stb3ItdjEtNmY2NTU2ZjJkZjczY2QwYTA4OTExN2FjY2IzN2U5YzU2ZTgyMDQ5ZjhiMzRkNTdmMmZhNDEyMzJmODJkNGQ0MQ==";
        String::from_utf8(STANDARD.decode(b64).unwrap_or_default()).unwrap_or_default()
    })
}


/// Fetch free models ordered by weekly popularity from OpenRouter.
async fn fetch_free_models() -> Result<Vec<String>> {
    let key = openrouter_key();
    let body = tokio::task::spawn_blocking(move || -> Result<String> {
        let out = std::process::Command::new("curl")
            .args(["-fsSL", "-H", &format!("Authorization: Bearer {key}"),
                   "https://openrouter.ai/api/frontend/models/find?order=top-weekly"])
            .output()?;
        if !out.status.success() {
            return Err(anyhow!("curl failed: {}", String::from_utf8_lossy(&out.stderr)));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }).await??;

    let v: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| anyhow!("OpenRouter JSON parse error: {e}"))?;

    let models: Vec<String> = v["data"]["models"]
        .as_array()
        .ok_or_else(|| anyhow!("unexpected OpenRouter response shape"))?
        .iter()
        .filter_map(|m| {
            let ep = m.get("endpoint")?;
            let pricing = ep.get("pricing")?;
            if pricing.get("prompt")?.as_str()? == "0" {
                Some(format!("{}:free", m["slug"].as_str()?))
            } else {
                None
            }
        })
        .collect();

    if models.is_empty() {
        return Err(anyhow!("no free models found on OpenRouter"));
    }
    Ok(models)
}

pub async fn list_free_models() -> Result<()> {
    eprintln!("🔍 Fetching free models from OpenRouter (by weekly popularity)...\n");
    let models = fetch_free_models().await?;
    println!("{:<4}  {}", "#", "MODEL");
    println!("{}", "-".repeat(60));
    for (i, id) in models.iter().enumerate() {
        let marker = if i == 0 { " ← selected" } else { "" };
        println!("{:<4}  {}{}", i + 1, id, marker);
    }
    println!("\nTotal: {} free models", models.len());
    Ok(())
}

/// Returns ordered list of models to try (first = best).
pub async fn resolve_models() -> Result<Vec<String>> {
    if let Ok(m) = env::var("FREECODE_MODEL") {
        return Ok(vec![m]);
    }
    eprintln!("  🔍 Fetching free models from OpenRouter...");
    match fetch_free_models().await {
        Ok(ids) if !ids.is_empty() => {
            eprintln!("  ✓ {} free models available, trying: {}", ids.len(), ids[0]);
            return Ok(ids);
        }
        Ok(_) => eprintln!("  ⚠ No free models found"),
        Err(e) => eprintln!("  ⚠ OpenRouter fetch failed: {e}"),
    }
    Err(anyhow!("Could not fetch free models from OpenRouter. Set FREECODE_MODEL to override."))
}

fn make_client() -> Client {
    let key = openrouter_key();
    Client::builder()
        .with_service_target_resolver_fn(move |mut st: ServiceTarget| {
            st.endpoint = Endpoint::from_owned("https://openrouter.ai/api/v1/");
            st.auth = AuthData::from_single(key.clone());
            // Force OpenAI adapter so genai doesn't misroute unknown model names to Ollama
            st.model = ModelIden::new(AdapterKind::OpenAI, st.model.model_name);
            Ok(st)
        })
        .build()
}

pub async fn run(cwd: &PathBuf, task: &str) -> Result<()> {
    let models = resolve_models().await?;
    let client = make_client();

    let git_ctx = {
        let looks_like_code_task = task.split_whitespace().count() > 3
            || task.contains('.')  // file extension
            || task.contains('/')  // path
            || ["fix", "refactor", "add", "implement", "debug", "test", "build", "run"]
                .iter().any(|w| task.to_lowercase().contains(w));
        if looks_like_code_task {
            tools::run_cmd(cwd, "git status --short 2>/dev/null; git diff --stat HEAD 2>/dev/null")
                .unwrap_or_default()
        } else {
            String::new()
        }
    };
    let first_msg = if git_ctx.trim().is_empty() {
        task.to_string()
    } else {
        format!("{task}\n\n<git_context>\n{git_ctx}\n</git_context>")
    };

    for (attempt, model) in models.iter().enumerate() {
        eprintln!("⚡ freecode  model={model}  cwd={}", cwd.display());
        if attempt == 0 { eprintln!("   {task}\n"); }

        match run_with_model(cwd, task, &client, model, first_msg.clone()).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                if models.len() > attempt + 1 {
                    eprintln!("  ⚠ {model} failed ({}), trying next model...", e);
                    continue;
                }
                return Err(e);
            }
        }
    }
    Ok(())
}

async fn run_with_model(cwd: &PathBuf, task: &str, client: &Client, model: &str, first_msg: String) -> Result<()> {

    let mut messages: Vec<ChatMessage> = vec![ChatMessage::user(first_msg)];
    let mut turn = 0usize;
    let mut files_changed = false;
    let requires_file_change = std::env::var("FREECODE_REQUIRE_FILE_CHANGE").map(|v| v == "1").unwrap_or_else(|_| {
        ["fix", "refactor", "add", "implement", "create", "write", "edit", "update", "patch"]
            .iter().any(|w| task.to_lowercase().contains(w))
    });

eprintln!("DEBUG: task='{}'", task.chars().take(50).collect::<String>());
    eprintln!("DEBUG: requires_file_change={}", requires_file_change);

    loop {
        if turn >= MAX_TURNS { eprintln!("(max turns reached)"); break; }

        if turn > 0 && turn % COMPRESS_AFTER == 0 {
            messages = compress(&client, &model, &messages, task).await?;
        }

        let reply = stream_reply(&client, &model, &messages).await?;

        if reply.is_empty() { break; }

        if !reply.contains("<think>") || !reply.contains("</think>") {
            messages.push(ChatMessage::assistant(&reply));
            messages.push(ChatMessage::user(
                "<result>ERROR: You must include a <think>...</think> block explaining your reasoning BEFORE calling any tool. Please try again.</result>"
            ));
            turn += 1;
            continue;
        }

        if reply.contains("<done>") {
            if requires_file_change && !files_changed {
                messages.push(ChatMessage::assistant(&reply));
                messages.push(ChatMessage::user(
                    "<result>You called <done> without making any file changes. \
                    You MUST apply a fix first. Use replace for a targeted edit, \
                    or if replace keeps failing, use write_file to rewrite the specific function/section.</result>"
                ));
                turn += 1;
                continue;
            }
            let msg = extract_between(&reply, "<done>", "</done>").unwrap_or("done");
            eprintln!("\n✓ {}", msg.trim());
            break;
        }

        let result = if let Some(pattern) = extract_attr(&reply, "grep", "pattern") {
            let path = extract_attr(&reply, "grep", "path");
            eprintln!("  🔍 grep {} (in {:?})", pattern, path);
            let r = tools::grep(cwd, &pattern, path.as_deref())?;
            log_cmd(cwd, &format!("grep {} {:?}", pattern, path), &r);
            r
        } else if let Some(pattern) = extract_attr(&reply, "find", "pattern") {
            eprintln!("  🔍 find {}", pattern);
            let r = tools::find(cwd, &pattern)?;
            log_cmd(cwd, &format!("find {}", pattern), &r);
            r
        } else if reply.contains("<ls ") || reply.contains("<ls/>") {
            let path = extract_attr(&reply, "ls", "path");
            eprintln!("  📁 ls {:?}", path);
            let r = tools::ls(cwd, path.as_deref())?;
            log_cmd(cwd, &format!("ls {:?}", path), &r);
            r
        } else if let Some(path) = extract_attr(&reply, "read_file", "path") {
            let start = extract_attr(&reply, "read_file", "start").and_then(|s| s.parse().ok());
            let end = extract_attr(&reply, "read_file", "end").and_then(|s| s.parse().ok());
            eprintln!("  📖 read_file {} (start={:?}, end={:?})", path, start, end);
            let r = tools::read_file(cwd, &path, start, end)?;
            log_cmd(cwd, &format!("read_file {}", path), &r);
            r
        } else if let Some(path) = extract_attr(&reply, "read_outline", "path") {
            eprintln!("  🌳 read_outline {}", path);
            let r = tools::read_outline(cwd, &path)?;
            log_cmd(cwd, &format!("read_outline {}", path), &r);
            r
        } else if let Some(path) = extract_attr(&reply, "write_file", "path") {
            let content = extract_between(&reply, ">", "</write_file>")
                .unwrap_or("")
                .trim_start_matches('\n')
                .to_string();
            let label = format!("write_file {path}");
            let dangerous = DANGEROUS.iter().any(|d| path.contains(d));
            let no_confirm = env::var("FREECODE_NO_CONFIRM").is_ok() || env::var("NANOCODE_NO_CONFIRM").is_ok();
            if dangerous && !no_confirm {
                eprint!("\n  ⚠ {label}  [y/N] ");
                std::io::stderr().flush()?;
                let mut inp = String::new();
                std::io::stdin().read_line(&mut inp)?;
                if !matches!(inp.trim().to_lowercase().as_str(), "y" | "yes") {
                    messages.push(ChatMessage::assistant(&reply));
                    messages.push(ChatMessage::user("<result>user declined</result>"));
                    turn += 1;
                    continue;
                }
            } else {
                eprintln!("  ✎ {label}");
            }
            let r = tools::write_file(cwd, &path, &content)?;
            log_cmd(cwd, &label, &r);
            if !r.starts_with("ERROR") {
                files_changed = true;
            }
            r
        } else if let Some(path) = extract_attr(&reply, "replace", "path") {
            let old = extract_tag_content(&reply, "old").unwrap_or("");
            let new = extract_tag_content(&reply, "new").unwrap_or("");
            eprintln!("  ⊕ replace in {}", path);
            let r = tools::replace(cwd, &path, old, new)?;
            log_cmd(cwd, &format!("replace in {}\n<old>\n{}\n</old>\n<new>\n{}\n</new>", path, old, new), &r);
            if !r.starts_with("ERROR") {
                files_changed = true;
            }
            r
        } else if let Some(cmd) = extract_attr(&reply, "run_cmd", "cmd") {
            let dangerous = DANGEROUS.iter().any(|d| cmd.contains(d));
            let no_confirm = env::var("FREECODE_NO_CONFIRM").is_ok() || env::var("NANOCODE_NO_CONFIRM").is_ok();
            if dangerous && !no_confirm {
                eprint!("\n  ⚠ $ {cmd}  [y/N] ");
                std::io::stderr().flush()?;
                let mut inp = String::new();
                std::io::stdin().read_line(&mut inp)?;
                if !matches!(inp.trim().to_lowercase().as_str(), "y" | "yes") {
                    log_cmd(cwd, &cmd, "declined by user");
                    messages.push(ChatMessage::assistant(&reply));
                    messages.push(ChatMessage::user("<result>user declined</result>"));
                    turn += 1;
                    continue;
                }
            } else {
                eprintln!("  $ {cmd}");
            }
            let r = tools::run_cmd(cwd, &cmd)?;
            log_cmd(cwd, &cmd, &r);
            r
        } else {
            eprintln!("{reply}");
            "ERROR: Unrecognized tool or format. Use <run_cmd>, <read_file>, <read_outline>, <grep>, <find>, <ls>, <write_file>, or <replace>.".to_string()
        };

        let truncated = truncate(&result, MAX_OUTPUT);
        eprintln!("{truncated}\n");
        messages.push(ChatMessage::assistant(&reply));
        messages.push(ChatMessage::user(format!("<result>\n{truncated}\n</result>")));
        turn += 1;
    }
    Ok(())
}

async fn stream_reply(client: &Client, model: &str, messages: &[ChatMessage]) -> Result<String> {
    use futures::StreamExt;
    let req = ChatRequest::new(messages.to_vec()).with_system(SYSTEM);
    let mut attempts = 0u32;
    let mut stream = loop {
        match client.exec_chat_stream(model, req.clone(), None).await {
            Ok(s) => break s,
            Err(e) if attempts < 3 && e.to_string().contains("429") => {
                eprintln!("  (rate limited, retrying in 10s...)");
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                attempts += 1;
            }
            Err(e) => return Err(e.into()),
        }
    };
    let mut full = String::new();
    let mut chars = 0usize;
    eprint!("  thinking");
    std::io::stderr().flush()?;
    while let Some(event) = stream.stream.next().await {
        match event {
            Ok(ChatStreamEvent::Chunk(chunk)) => {
                full.push_str(&chunk.content);
                chars += chunk.content.len();
                if chars % 50 < chunk.content.len() {
                    eprint!(".");
                    std::io::stderr().flush()?;
                }
            }
            Err(e) => return Err(e.into()),
            _ => {}
        }
    }
    eprint!("\r              \r");
    std::io::stderr().flush()?;
    Ok(full.trim().to_string())
}

async fn compress(client: &Client, model: &str, messages: &[ChatMessage], task: &str) -> Result<Vec<ChatMessage>> {
    let mut prev_summary = String::new();
    if let Some(first) = messages.first() {
        let content = content_str(first);
        if let Some(s) = extract_between(&content, "<session_summary>", "</session_summary>") {
            prev_summary = s.trim().to_string();
        }
    }

    let history = messages.iter().map(|m| format!("{:?}: {}", m.role, content_str(m))).collect::<Vec<_>>().join("\n");

    let prompt = if prev_summary.is_empty() {
        format!("Original task: {task}\n\nHistory:\n{history}\n\n{SUMMARIZATION_PROMPT}")
    } else {
        format!("Original task: {task}\n\n<previous-summary>\n{prev_summary}\n</previous-summary>\n\nHistory:\n{history}\n\n{UPDATE_SUMMARIZATION_PROMPT}")
    };

    let summary_req = ChatRequest::new(vec![
        ChatMessage::user(prompt)
    ]);

    let res = client.exec_chat(model, summary_req, None).await?;
    let summary = res.content_text_as_str().unwrap_or("(summary unavailable)").to_string();

    eprintln!("\n[context compressed]\n");
    Ok(vec![
        ChatMessage::user(format!("Original task: {task}\n\n<session_summary>\n{summary}\n</session_summary>\n\nContinue where you left off."))
    ])
}

fn content_str(m: &ChatMessage) -> String {
    match &m.content {
        genai::chat::MessageContent::Text(t) => t.clone(),
        genai::chat::MessageContent::Parts(_) => "(parts)".into(),
        _ => "(non-text)".into(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { return s.to_string(); }
    let half = max / 2;
    
    let mut head_end = half;
    while head_end > 0 && !s.is_char_boundary(head_end) {
        head_end -= 1;
    }
    let mut head = &s[..head_end];
    if let Some(pos) = head.rfind('\n') {
        head = &head[..pos];
    }

    let mut tail_start = s.len() - half;
    while tail_start < s.len() && !s.is_char_boundary(tail_start) {
        tail_start += 1;
    }
    let mut tail = &s[tail_start..];
    if let Some(pos) = tail.find('\n') {
        tail = &tail[pos + 1..];
    }

    format!("{}\n\n[TRUNCATED — {} bytes omitted]\n\n{}", head, s.len() - head.len() - tail.len(), tail)
}

fn log_cmd(cwd: &PathBuf, cmd: &str, result: &str) {
    use std::fs::OpenOptions;
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(cwd.join(".freecode.log")) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "[{ts}] $ {cmd}\n{result}\n");
    }
}

fn extract_attr(s: &str, tag: &str, attr: &str) -> Option<String> {
    let re = regex::Regex::new(&format!(r#"<{} [^>]*?{}="([^"]*)""#, tag, attr)).ok()?;
    re.captures(s).and_then(|cap| cap.get(1)).map(|m| m.as_str().to_string())
}

fn extract_between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = s.find(open)? + open.len();
    let b = s[a..].find(close)?;
    Some(&s[a..a+b])
}

fn extract_tag_content<'a>(s: &'a str, tag: &str) -> Option<&'a str> {
    let start_re = regex::Regex::new(&format!(r#"<{}\s*>"#, tag)).ok()?;
    let end_re = regex::Regex::new(&format!(r#"</{}\s*>"#, tag)).ok()?;
    
    let start_match = start_re.find(s)?;
    let start_idx = start_match.end();
    let end_match = end_re.find(&s[start_idx..])?;
    let end_idx = start_idx + end_match.start();
    
    let mut content = &s[start_idx..end_idx];
    while content.starts_with('\n') {
        content = &content[1..];
    }
    while content.ends_with('\n') {
        content = &content[..content.len()-1];
    }
    Some(content)
}
