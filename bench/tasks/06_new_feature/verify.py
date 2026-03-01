import subprocess, sys
result = subprocess.run([sys.executable, "-c", """
import importlib.util
spec = importlib.util.spec_from_file_location("sol", "utils.py")
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
assert m.is_palindrome("racecar") == True
assert m.is_palindrome("A man a plan a canal Panama") == True
assert m.is_palindrome("hello") == False
print("PASS")
"""], capture_output=True, text=True)
print(result.stdout.strip() or result.stderr.strip())
sys.exit(0 if "PASS" in result.stdout else 1)
