import subprocess, sys
result = subprocess.run([sys.executable, "-c", """
import importlib.util, inspect
spec = importlib.util.spec_from_file_location("sol", "messy.py")
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
fns = [v for k,v in inspect.getmembers(m, inspect.isfunction)]
assert fns, "no functions found"
fn = fns[0]
out = fn(15)
assert out[2] == "Fizz" and out[4] == "Buzz" and out[14] == "FizzBuzz"
print("PASS")
"""], capture_output=True, text=True)
print(result.stdout.strip() or result.stderr.strip())
sys.exit(0 if "PASS" in result.stdout else 1)
