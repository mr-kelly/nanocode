import sys
if not __import__('os').path.exists("sorted.txt"):
    print("FAIL: sorted.txt not found"); sys.exit(1)
nums = [int(x) for x in open("sorted.txt").read().split() if x]
if nums == sorted(nums) and sorted(nums) == sorted([3,1,4,1,5,9,2,6]):
    print(f"PASS: {nums}"); sys.exit(0)
else:
    print(f"FAIL: {nums}"); sys.exit(1)
