import os, sys
if not os.path.exists("answer.txt"):
    print("FAIL: answer.txt not found"); sys.exit(1)
n = int(open("answer.txt").read().strip())
# count all files at verify time (including answer.txt itself)
actual = len([f for f in os.listdir(".") if os.path.isfile(f)])
# model may have counted before or after writing answer.txt — accept ±1
if abs(n - actual) <= 1:
    print(f"PASS: answered {n}, actual {actual}"); sys.exit(0)
else:
    print(f"FAIL: got {n}, actual {actual}"); sys.exit(1)
