<p align="center">
  <img src="logo.svg" width="96" height="96" alt="freecode logo"/>
</p>

<h1 align="center">freecode</h1>

<p align="center">
  A minimal autonomous coding agent in Rust.<br/>
  <strong>~300 lines. $0. Free forever. Always picks the best free model automatically.</strong>
</p>

<p align="center">
  <a href="https://github.com/mr-kelly/freecode/releases"><img src="https://img.shields.io/github/v/release/mr-kelly/freecode?color=facc15&labelColor=0f0f0f" alt="release"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-facc15?labelColor=0f0f0f" alt="license"/></a>
  <img src="https://img.shields.io/badge/built_with-Rust-facc15?labelColor=0f0f0f" alt="rust"/>
</p>

---

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/mr-kelly/freecode/main/install.sh | bash
```

```bash
brew tap mr-kelly/tap && brew install freecode   # Homebrew
```

```bash
cargo install --path .                           # from source
```

---

## Usage

```bash
freecode "fix the failing tests in src/"
freecode "add is_palindrome to utils.py"
freecode "refactor main.rs to split the agent loop into its own module"
```

```bash
# pipe JSON for programmatic use
echo '{"prompt": "fix the bug", "cwd": "./myproject"}' | freecode
```

---

## How the free model works

freecode has no hardcoded model. On every run, it fetches the **real-time popularity ranking** from OpenRouter and picks the #1 free model by weekly usage across all users.

```
startup
  └─ GET openrouter.ai/api/frontend/models/find?order=top-weekly
       └─ filter: pricing.prompt == "0"
            └─ try #1 → 429/401? → try #2 → fail? → try #3 → ...
```

If the top model is rate-limited or down, it automatically falls back to #2, #3, and so on — no intervention needed.

```bash
freecode --list-free   # see current ranking
```

```
#     MODEL
------------------------------------------------------------
1     arcee-ai/trinity-large-preview:free  ← selected
2     stepfun/step-3.5-flash:free
3     z-ai/glm-4.5-air:free
4     nvidia/nemotron-3-nano-30b-a3b:free
5     openai/gpt-oss-120b:free
...
```

A built-in OpenRouter key is bundled — run with zero setup. Set `OPENROUTER_API_KEY` to use your own.

---

## How it works

| | |
|---|---|
| **Turns** | Up to 40 per task |
| **Tools** | `run_cmd` (shell) · `write_file` · `apply_patch` |
| **Safety** | Dangerous commands (`rm`, `sudo`, `git push` …) require confirmation |
| **Context** | Output truncated at 8 000 bytes · history compressed every 10 turns |
| **Git** | Auto-seeds `git status` + `git diff` at start |
| **Log** | All commands logged to `.freecode.log` |

---

## Benchmark

Tested with `arcee-ai/trinity-large-preview:free` (top free model on OpenRouter at time of writing):

| # | Task | Description | Result |
|---|------|-------------|--------|
| 01 | count_files | Count files in dir, write to answer.txt | ✅ |
| 02 | hello_world | Create hello.txt with exact content | ✅ |
| 03 | fizzbuzz | Write fizzbuzz function from scratch | ✅ |
| 04 | bugfix | Find and fix off-by-one bug | ✅ |
| 05 | refactor | Refactor messy code, preserve behavior | ✅ |
| 06 | new_feature | Add `is_palindrome` to existing module | ✅ |
| 07 | file_ops | Create JSON file + Python reader script | ✅ |
| 08 | sort_numbers | Sort numbers from file, write to sorted.txt | ✅ |
| 09 | word_count | Count words in file, write to count.txt | ✅ |
| 10 | rename_file | Rename a file | ✅ |

**10/10** with a free model. Run it yourself:

```bash
./bench/run.sh
```

---

## Override model or provider

By default, freecode uses a built-in OpenRouter key that only works with free models — no setup needed.

To use your own key or a paid model:

```bash
export OPENROUTER_API_KEY="sk-or-v1-..."   # your own OpenRouter key
export FREECODE_MODEL="claude-opus-4"       # pin any model available on OpenRouter
```

---

<p align="center">
  MIT License · <a href="https://openrouter.ai">openrouter.ai</a> · <a href="https://github.com/mr-kelly/freecode/issues">issues</a>
</p>
