import subprocess, sys
result = subprocess.run([sys.executable, "-c", """
import importlib.util, sys
spec = importlib.util.spec_from_file_location("sol", "solution.py")
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
out = m.fizzbuzz(15)
assert out[0] == "1", f"got {out[0]}"
assert out[2] == "Fizz", f"got {out[2]}"
assert out[4] == "Buzz", f"got {out[4]}"
assert out[14] == "FizzBuzz", f"got {out[14]}"
print("PASS")
"""], capture_output=True, text=True)
print(result.stdout.strip() or result.stderr.strip())
sys.exit(0 if "PASS" in result.stdout else 1)
