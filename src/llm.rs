use anyhow::{anyhow, Result};
use genai::{
    adapter::AdapterKind,
    chat::{ChatMessage, ChatRequest, ChatStreamEvent},
    resolver::{AuthData, Endpoint, ServiceTargetResolver},
    Client, ModelIden, ServiceTarget,
};
use std::{env, io::Write, path::PathBuf};
use crate::tools;

const PROVIDERS: &[(&str, &str)] = &[
    ("OPENAI_API_KEY",    "gpt-5.2"),
    ("ANTHROPIC_API_KEY", "claude-sonnet-4.6"),
    ("GEMINI_API_KEY",    "gemini-3.0-flash"),
    ("GROQ_API_KEY",      "llama-3.1-8b-instant"),
    ("DEEPSEEK_API_KEY",  "deepseek-chat"),
    ("XAI_API_KEY",       "grok-beta"),
];

const DANGEROUS: &[&str] = &[
    "rm ", "rm\t", ":(){:|:&};:",
    "sudo ", "su ",
    "git push", "git reset --hard", "git clean -f",
    "dd ", "mkfs", "fdisk", "parted",
    "shutdown", "reboot", "halt",
    "curl | sh", "wget | sh", "bash <(",
];

const MAX_OUTPUT: usize = 8000; // truncate long command output
const MAX_TURNS:  usize = 40;
const COMPRESS_AFTER: usize = 10; // compress history after N turns

const SYSTEM: &str = "\
You are Nanocode — a fast, autonomous terminal agent.

TOOLS — output exactly one per turn, then wait for the result:

<run_cmd cmd=\"ls -la\" />
  Run any single-line shell command. For exploration, builds, git, etc.

<write_file path=\"path/to/file\">
content line 1
content line 2
</write_file>
  Write a file with multi-line content. Always use this for file creation/editing.
  Never use shell heredocs or echo redirects for multi-line content.

<apply_patch>
--- a/path/to/file
+++ b/path/to/file
@@ -1,4 +1,4 @@
 context
-old line
+new line
 context
</apply_patch>
  Apply a unified diff patch. Prefer this over write_file for targeted edits to existing files.

<done>summary</done>
  Signal task completion.

STRATEGY:
1. Explore first (ls, cat, git status), then act.
2. One tool per turn. Read output before next step.
3. For large files, use grep/sed to read only relevant sections, not cat the whole file.
4. Use apply_patch for targeted edits. Use write_file for new files or full rewrites.
5. Never use shell heredocs or echo redirects for multi-line content.
6. If a command fails, diagnose and retry.
7. IMPORTANT: You MUST make at least one file change (apply_patch or write_file) before calling <done>.
8. When done: <done>concise summary</done>

OUTPUT: one tool call only. Nothing else. No markdown.
";

pub fn resolve_model() -> Result<String> {
    if let Ok(m) = env::var("NANOCODE_MODEL").or_else(|_| env::var("OPENAI_MODEL")) {
        return Ok(m);
    }
    for (key, model) in PROVIDERS {
        if env::var(key).is_ok() { return Ok(model.to_string()); }
    }
    Err(anyhow!("No provider. Set one of: {}", PROVIDERS.iter().map(|(k,_)| *k).collect::<Vec<_>>().join(", ")))
}

fn make_client() -> Client {
    if let Ok(base_url) = env::var("OPENAI_BASE_URL") {
        let resolver = ServiceTargetResolver::from_resolver_fn(
            move |st: ServiceTarget| -> std::result::Result<ServiceTarget, genai::resolver::Error> {
                Ok(ServiceTarget {
                    endpoint: Endpoint::from_owned(format!("{}/", base_url.trim_end_matches('/'))),
                    auth: AuthData::from_env("OPENAI_API_KEY"),
                    model: ModelIden::new(AdapterKind::OpenAI, st.model.model_name),
                })
            },
        );
        Client::builder().with_service_target_resolver(resolver).build()
    } else {
        Client::default()
    }
}

pub async fn run(cwd: &PathBuf, task: &str) -> Result<()> {
    let model = resolve_model()?;
    eprintln!("⚡ nanocode  model={model}  cwd={}", cwd.display());
    eprintln!("   {task}\n");

    let client = make_client();

    // 2. Seed with git context
    let git_ctx = tools::run_cmd(cwd, "git status --short 2>/dev/null; git diff --stat HEAD 2>/dev/null")
        .unwrap_or_default();
    let first_msg = if git_ctx.trim().is_empty() {
        task.to_string()
    } else {
        format!("{task}\n\n<git_context>\n{git_ctx}\n</git_context>")
    };

    let mut messages: Vec<ChatMessage> = vec![ChatMessage::user(first_msg)];
    let mut turn = 0usize;
    let mut files_changed = false;

    loop {
        if turn >= MAX_TURNS { eprintln!("(max turns reached)"); break; }

        // 5. Compress history every COMPRESS_AFTER turns
        if turn > 0 && turn % COMPRESS_AFTER == 0 {
            messages = compress(&client, &model, &messages, task).await?;
        }

        // 4. Streaming output
        let reply = stream_reply(&client, &model, &messages).await?;

        if reply.is_empty() { break; }

        if reply.contains("<done>") {
            if !files_changed {
                // model tried to finish without making any changes — push back
                messages.push(ChatMessage::assistant(&reply));
                messages.push(ChatMessage::user("<result>You called <done> without making any file changes. You MUST apply a fix using apply_patch or write_file before finishing.</result>"));
                turn += 1;
                continue;
            }
            let msg = extract_between(&reply, "<done>", "</done>").unwrap_or("done");
            eprintln!("\n✓ {}", msg.trim());
            break;
        }

        let result = if let Some(path) = extract_attr(&reply, "write_file", "path") {
            let content = extract_between(&reply, ">", "</write_file>")
                .unwrap_or("")
                .trim_start_matches('\n')
                .to_string();
            let label = format!("write_file {path}");
            let dangerous = DANGEROUS.iter().any(|d| path.contains(d));
            if dangerous {
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
            files_changed = true;
            r
        } else if reply.contains("<apply_patch>") {
            let diff = extract_between(&reply, "<apply_patch>", "</apply_patch>").unwrap_or("").to_string();
            eprintln!("  ⊕ apply_patch");
            let r = tools::apply_patch(cwd, &diff)?;
            log_cmd(cwd, "apply_patch", &r);
            files_changed = true;
            r
        } else if let Some(cmd) = extract_attr(&reply, "run_cmd", "cmd") {
            let dangerous = DANGEROUS.iter().any(|d| cmd.contains(d));
            if dangerous {
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
            break;
        };

        let truncated = truncate(&result, MAX_OUTPUT);
        eprintln!("{truncated}\n");
        messages.push(ChatMessage::assistant(&reply));
        messages.push(ChatMessage::user(format!("<result>\n{truncated}\n</result>")));
        turn += 1;
    }
    Ok(())
}

// Stream tokens silently to collect full reply, then parse and display cleanly
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
        if let Ok(ChatStreamEvent::Chunk(chunk)) = event {
            full.push_str(&chunk.content);
            chars += chunk.content.len();
            // Show a dot every ~50 chars so user knows it's alive
            if chars % 50 < chunk.content.len() {
                eprint!(".");
                std::io::stderr().flush()?;
            }
        }
    }
    eprint!("\r              \r");
    std::io::stderr().flush()?;
    Ok(full.trim().to_string())
}

// 5. Compress old messages into a summary to save context
async fn compress(client: &Client, model: &str, messages: &[ChatMessage], task: &str) -> Result<Vec<ChatMessage>> {
    let history = messages.iter().map(|m| format!("{:?}: {}", m.role, content_str(m))).collect::<Vec<_>>().join("\n");
    let summary_req = ChatRequest::new(vec![
        ChatMessage::user(format!(
            "Summarize this agent session concisely for context. Original task: {task}\n\nHistory:\n{history}"
        ))
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
    let cut = &s[..max];
    // cut at last newline to avoid broken lines
    let end = cut.rfind('\n').unwrap_or(max);
    format!("{}\n[TRUNCATED — {} bytes omitted]", &s[..end], s.len() - end)
}

fn log_cmd(cwd: &PathBuf, cmd: &str, result: &str) {
    use std::fs::OpenOptions;
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(cwd.join(".nanocode.log")) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "[{ts}] $ {cmd}\n{result}\n");
    }
}

fn extract_attr(s: &str, tag: &str, attr: &str) -> Option<String> {
    let pos = s.find(&format!("<{tag} "))?;
    let rest = &s[pos..];
    let a = rest.find(&format!("{attr}=\""))? + attr.len() + 2;
    let b = rest[a..].find('"')?;
    Some(rest[a..a+b].to_string())
}

fn extract_between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = s.find(open)? + open.len();
    let b = s[a..].find(close)?;
    Some(&s[a..a+b])
}
