import os, sys
if not os.path.exists("answer.txt"):
    print("FAIL: answer.txt not found"); sys.exit(1)
try:
    n = int(open("answer.txt").read().strip())
except Exception:
    print("FAIL: could not parse int")
    sys.exit(1)
# count all files at verify time, ignoring .freecode.log which is created at runtime
actual = len([f for f in os.listdir(".") if os.path.isfile(f) and f != ".freecode.log"])
if abs(n - actual) <= 1:
    print(f"PASS: answered {n}, actual {actual}"); sys.exit(0)
else:
    print(f"FAIL: got {n}, actual {actual}"); sys.exit(1)
