import os, sys
if os.path.exists("new_name.txt") and not os.path.exists("old_name.txt"):
    print("PASS"); sys.exit(0)
elif not os.path.exists("new_name.txt"):
    print("FAIL: new_name.txt not found"); sys.exit(1)
else:
    print("FAIL: old_name.txt still exists"); sys.exit(1)
