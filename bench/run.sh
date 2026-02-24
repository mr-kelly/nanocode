#!/usr/bin/env bash
# Usage: OPENAI_MODEL=gemini-3-flash ./bench/run.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
NANOCODE="${NANOCODE:-$REPO_DIR/target/debug/nanocode}"
TASKS_DIR="$SCRIPT_DIR/tasks"
PASS=0; FAIL=0; TOTAL=0

for task_dir in "$TASKS_DIR"/*/; do
    name=$(basename "$task_dir")
    task_file="$task_dir/task.txt"
    verify_file="$task_dir/verify.py"
    [ -f "$task_file" ] || continue

    echo "━━━ $name ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Run nanocode in a temp copy of the task dir
    tmpdir=$(mktemp -d)
    cp "$task_dir"/* "$tmpdir/" 2>/dev/null
    cd "$tmpdir"

    prompt=$(cat task.txt)
    "$NANOCODE" "$prompt" 2>&1

    # Verify
    if python3 verify.py 2>&1; then
        echo "  → ✅ PASS"
        PASS=$((PASS+1))
    else
        echo "  → ❌ FAIL"
        FAIL=$((FAIL+1))
    fi
    TOTAL=$((TOTAL+1))

    cd - > /dev/null
    rm -rf "$tmpdir"
    echo
    sleep 8  # avoid rate limits
done

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Results: $PASS/$TOTAL passed  ($FAIL failed)"
