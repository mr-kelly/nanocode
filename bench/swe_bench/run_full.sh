#!/bin/bash
cargo build --release
python3 bench/swe_bench/run_inference.py --out bench/swe_bench/predictions_full.jsonl > full_inference.log 2>&1
