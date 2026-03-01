import sys
if not __import__('os').path.exists("count.txt"):
    print("FAIL: count.txt not found"); sys.exit(1)
n = int(open("count.txt").read().strip())
expected = len(open("input.txt").read().split())
if n == expected:
    print(f"PASS: {n} words"); sys.exit(0)
else:
    print(f"FAIL: got {n}, expected {expected}"); sys.exit(1)
