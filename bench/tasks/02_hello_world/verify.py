import os, sys
if not os.path.exists("hello.txt"):
    print("FAIL: hello.txt not found"); sys.exit(1)
content = open("hello.txt").read().strip()
if content == "Hello, World!":
    print("PASS"); sys.exit(0)
else:
    print(f"FAIL: got {repr(content)}"); sys.exit(1)
