#!/usr/bin/env python3
"""
SWE-bench Lite inference for nanocode.
Clones each repo, runs nanocode on the issue, captures the resulting git diff as a patch.

Usage:
    pip install datasets gitpython
    python run_inference.py --out predictions.jsonl [--limit 10] [--instance sympy__sympy-20590]
"""
import argparse, json, os, subprocess, sys, tempfile
from pathlib import Path

NANOCODE = str(Path(__file__).parent.parent / "target" / "debug" / "nanocode")

def run_instance(instance: dict, tmpdir: str) -> dict | None:
    repo_url = f"https://github.com/{instance['repo']}.git"
    commit   = instance["base_commit"]
    issue    = instance["problem_statement"]
    iid      = instance["instance_id"]

    repo_dir = os.path.join(tmpdir, iid)
    print(f"\n{'='*60}\n{iid}", flush=True)

    # Clone and checkout base commit
    r = subprocess.run(["git", "clone", "--quiet", repo_url, repo_dir], capture_output=True)
    if r.returncode != 0:
        print(f"  clone failed: {r.stderr.decode()[:200]}")
        return None
    subprocess.run(["git", "checkout", "--quiet", commit], cwd=repo_dir, capture_output=True)

    # Run nanocode
    prompt = (
        f"Fix the following GitHub issue by editing the source code directly.\n\n"
        f"Issue:\n{issue}\n\n"
        f"Instructions:\n"
        f"- Read the relevant source files with grep/sed to understand the code\n"
        f"- Search ALL occurrences of the pattern you're fixing (grep -rn) before patching\n"
        f"- Apply the minimal fix using apply_patch (preferred) or write_file (new files only)\n"
        f"- Do NOT install packages, run tests, or try to execute the code\n"
        f"- Do NOT use python3 -c with single-quoted strings (shell quoting issues)\n"
        f"- For large existing files, you MUST use apply_patch, not write_file\n"
        f"- Make sure your fix covers ALL relevant locations, not just the first one found\n"
        f"- When done, call <done>description of fix</done>"
    )
    env = {**os.environ, "NANOCODE_NO_CONFIRM": "1"}
    subprocess.run([NANOCODE, prompt], cwd=repo_dir, env=env, timeout=300)

    # Capture diff
    diff = subprocess.run(
        ["git", "diff", "--no-color"],
        cwd=repo_dir, capture_output=True, text=True
    ).stdout

    if not diff.strip():
        print("  (no changes made)")
        return {"instance_id": iid, "model_patch": "", "model_name_or_path": "nanocode"}

    print(f"  patch: {len(diff)} bytes")
    return {"instance_id": iid, "model_patch": diff, "model_name_or_path": "nanocode"}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out",      default="predictions.jsonl")
    ap.add_argument("--limit",    type=int, default=0, help="0 = all 300")
    ap.add_argument("--instance", default="", help="run single instance id")
    ap.add_argument("--ids",      default="", help="comma-separated instance ids")
    args = ap.parse_args()

    from datasets import load_dataset
    ds = load_dataset("princeton-nlp/SWE-bench_Lite", split="test")

    if args.instance:
        instances = [x for x in ds if x["instance_id"] == args.instance]
    elif args.ids:
        id_set = set(args.ids.split(","))
        instances = [x for x in ds if x["instance_id"] in id_set]
    elif args.limit:
        instances = list(ds)[:args.limit]
    else:
        instances = list(ds)

    print(f"Running {len(instances)} instances → {args.out}")

    done = set()
    if os.path.exists(args.out):
        with open(args.out) as f:
            for line in f:
                done.add(json.loads(line)["instance_id"])
        print(f"  ({len(done)} already done, skipping)")

    with open(args.out, "a") as out_f, tempfile.TemporaryDirectory() as tmpdir:
        for inst in instances:
            if inst["instance_id"] in done:
                continue
            try:
                result = run_instance(inst, tmpdir)
                if result:
                    out_f.write(json.dumps(result) + "\n")
                    out_f.flush()
            except subprocess.TimeoutExpired:
                print(f"  TIMEOUT: {inst['instance_id']}")
            except Exception as e:
                print(f"  ERROR: {e}")

    print(f"\nDone. Predictions in {args.out}")
    print("Evaluate with:")
    print(f"  python -m swebench.harness.run_evaluation \\")
    print(f"    --dataset_name princeton-nlp/SWE-bench_Lite \\")
    print(f"    --predictions_path {args.out} \\")
    print(f"    --max_workers 4 \\")
    print(f"    --run_id nanocode_run1")


if __name__ == "__main__":
    main()
