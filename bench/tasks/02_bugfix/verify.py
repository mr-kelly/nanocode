import subprocess, sys
result = subprocess.run([sys.executable, "-c", """
import importlib.util
spec = importlib.util.spec_from_file_location("sol", "buggy.py")
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
assert m.sum_list([1,2,3,4,5]) == 15, f"sum_list failed: {m.sum_list([1,2,3,4,5])}"
assert m.average([1,2,3]) == 2.0, f"average failed: {m.average([1,2,3])}"
print("PASS")
"""], capture_output=True, text=True)
print(result.stdout.strip() or result.stderr.strip())
sys.exit(0 if "PASS" in result.stdout else 1)
