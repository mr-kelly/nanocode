import subprocess, sys, json, os
# check data.json exists and valid
assert os.path.exists("data.json"), "data.json missing"
data = json.load(open("data.json"))
assert isinstance(data, list) and len(data) == 5, "need 5 items"
item3 = next((x for x in data if x.get("id") == 3), None)
assert item3 and "name" in item3, "no item with id=3"
# check read_data.py runs and prints the name
result = subprocess.run([sys.executable, "read_data.py"], capture_output=True, text=True)
assert item3["name"].strip() in result.stdout, f"expected '{item3['name']}' in output, got: {result.stdout!r}"
print("PASS")
