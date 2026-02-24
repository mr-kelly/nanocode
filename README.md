# nanocode

A minimal autonomous coding agent in Rust. Give it a task, it figures out the rest using shell commands.

```
prompt → LLM → run_cmd / write_file → observe → repeat
```

## Install

```bash
cargo install --path .
# or build manually
cargo build --release && cp target/release/nanocode ~/.local/bin/
```

## Usage

```bash
nanocode "fix the failing tests in src/"
nanocode "add a is_palindrome function to utils.py"
nanocode "refactor main.rs to split the agent loop into its own module"
```

Or pipe JSON for programmatic use:

```bash
echo '{"prompt": "fix the bug", "cwd": "./myproject"}' | nanocode
```

## Providers

Set one API key and nanocode picks it up automatically:

```bash
export OPENAI_API_KEY="..."       # uses gpt-5.2 by default
export ANTHROPIC_API_KEY="..."    # uses claude-sonnet-4.6
export GEMINI_API_KEY="..."       # uses gemini-3.0-flash
export GROQ_API_KEY="..."
export DEEPSEEK_API_KEY="..."
export XAI_API_KEY="..."
```

Override model or endpoint:

```bash
export OPENAI_MODEL=gpt-4.1
export NANOCODE_MODEL=claude-opus-4
export OPENAI_BASE_URL=https://my-proxy.example.com/v1
```

## How it works

- Up to 40 turns per task
- Two tools: `run_cmd` (shell) and `write_file` (multi-line content)
- Dangerous commands (`rm`, `sudo`, `git push`, etc.) require confirmation
- Output truncated at 8000 bytes to keep context manageable
- History compressed every 10 turns via LLM summarization
- Git context auto-seeded at start (`git status`, `git diff --stat HEAD`)
- All commands logged to `.nanocode.log`

## Benchmark

Tested on a custom 5-task suite (`bench/`) with `gemini-3-flash`:

| Task | Description | Result |
|------|-------------|--------|
| 01_fizzbuzz | Write fizzbuzz function from scratch | ✅ PASS |
| 02_bugfix | Find and fix off-by-one bug | ✅ PASS |
| 03_refactor | Refactor messy code, preserve behavior | ✅ PASS |
| 04_new_feature | Add `is_palindrome` to existing module | ✅ PASS |
| 05_file_ops | Create JSON file + Python reader script | ✅ PASS |

**5/5 (100%)** with `gemini-3-flash` (free tier).

Run it yourself:

```bash
OPENAI_MODEL=gemini-3-flash ./bench/run.sh
```

## Simpler benchmarks to try

If you want a quick sanity check without Docker or harnesses:

- **HumanEval** — 164 Python function-completion tasks, just run `python evaluate.py`
- **MBPP** — 374 crowd-sourced Python problems, similar format
- **Our bench/** — the 5 tasks above, zero dependencies beyond Python 3

For serious leaderboard comparison, [Terminal-Bench 2.0](https://tbench.ai) via [Harbor](https://harborframework.com) is the standard (requires Docker).
