# AGENTS.md — nanocode SWE-bench Experiment Log

## Overview

nanocode is a minimal Rust coding agent (~300 lines) with two tools: `run_cmd` (shell) and `write_file`/`apply_patch` (file edits). This document records the SWE-bench Lite evaluation experience, failure patterns, and iterative improvements.

## SWE-bench Lite Results (68/300 attempted)

Model: `gemini-3-flash` (free tier, via OpenAI-compatible proxy)

| Metric | Value |
|--------|-------|
| Attempted | 68 / 300 |
| Generated patch | 52 / 68 (76%) |
| No patch produced | 16 / 68 (24%) |
| Passed (resolved) | 19 / 68 (27.9%) |
| Pass rate (of with-patch) | 19 / 52 (36.5%) |

Extrapolated to full 300: ~**28%** if rate holds.

For reference, Claude Code + Claude Opus 4.6 scores 58% on the same benchmark.

---

## Failure Taxonomy

### 1. No patch produced (24% of attempts)
Model analyzes the issue, identifies the fix, but calls `<done>` without executing any file change.

**Root cause**: Model treats analysis as sufficient. Common on complex ORM/migration issues where the fix requires understanding multiple interacting components.

**Fix applied**: Added `files_changed` guard — if model calls `<done>` without any write, it gets pushed back with an error message forcing it to make a change.

**Remaining issue**: Model sometimes exhausts max_turns (40) retrying malformed patches without ever succeeding.

---

### 2. Incomplete patch (most FAIL cases)
Model finds one occurrence of the bug but misses others in the same file or related files.

**Example**: `astropy__astropy-14365` — needed `re.IGNORECASE` in two places; model only patched the first.

**Fix applied**: Added to inference prompt: *"Search ALL occurrences with grep -rn before patching. Make sure your fix covers all relevant locations."*

**Remaining issue**: Model still sometimes stops after the first grep hit.

---

### 3. Wrong file / wrong approach
Model patches a plausible-looking file but the actual fix is elsewhere.

**Example**: `django__django-11564` — model patched `static.py`, gold patch is in `conf/__init__.py`.

**Root cause**: Issue description mentions a symptom visible in one file, but the root cause is in another. Model follows the symptom.

**No fix yet**: Would require deeper code understanding or retrieval-augmented context.

---

### 4. Patch format errors
`git apply` fails due to wrong context lines, off-by-one line numbers, or indentation mismatches in the diff.

**Fix applied**: Switched from `patch -p1` to `git apply --whitespace=fix` (more forgiving). Model now receives the full error message and can retry.

**Remaining issue**: After 2-3 failed attempts, model sometimes gives up and writes a `.fixed` file instead of the original.

---

### 5. Over-rewriting large files
Model uses `write_file` to rewrite an entire 1000-line file instead of a targeted patch.

**Example**: `django__django-11019` — model rewrote all of `widgets.py` (52KB patch).

**Fix applied**: `write_file` now refuses to overwrite files with >300 lines, returning an error that forces the model to use `apply_patch`.

---

## Iterative Improvements Made

| Change | Impact |
|--------|--------|
| `files_changed` guard on `<done>` | Converted several NO PATCH → PASS |
| `git apply` instead of `patch -p1` | Fewer format-related failures |
| write_file blocked for >300 line files | Prevents 50KB rewrites |
| Prompt: search all occurrences | Marginal improvement on multi-location bugs |
| Line continuation collapse in run_cmd | Fixed `grep -r \` shell errors |

---

## Running the Benchmark

```bash
# Inference (generates patches)
OPENAI_MODEL=gemini-3-flash python3 bench/swe_bench/run_inference.py \
  --out bench/swe_bench/predictions.jsonl \
  --limit 50   # or --instance <id> for single

# Evaluation (requires Docker, ~2min/instance)
python3.12 -m swebench.harness.run_evaluation \
  --dataset_name princeton-nlp/SWE-bench_Lite \
  --predictions_path bench/swe_bench/predictions.jsonl \
  --max_workers 4 \
  --report_dir logs/swe_bench \
  --run_id nanocode_run1
```

Results JSON → `logs/swe_bench/`
Docker build/eval logs → `logs/run_evaluation/`

---

## Known Remaining Issues

1. **Complex multi-file bugs** — model can't reason across files without explicit retrieval
2. **Patch retry loop** — after 3 failed `git apply` attempts, model sometimes gives up
3. **Feature requests vs bug fixes** — feature requests require understanding the full intended behavior, not just fixing a crash
4. **Rate limits** — gemini-3-flash free tier hits 429 during long runs; retry logic added but stream-level errors still cause failures

## Next Steps

- [ ] Add file-level context injection: before the agent starts, auto-grep the repo for symbols mentioned in the issue and include top matches in the prompt
- [ ] Smarter retry: if `apply_patch` fails 3 times, auto-switch to `write_file` on just the relevant function (extracted via grep)
- [ ] Run full 300 instances for official score
- [ ] Try with a stronger model (Claude Opus 4.6 or GPT-5.3) to separate agent quality from model quality
