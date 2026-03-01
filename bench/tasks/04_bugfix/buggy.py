def sum_list(nums):
    total = 0
    for i in range(len(nums) - 1):  # bug: off-by-one, should be range(len(nums))
        total += nums[i]
    return total

def average(nums):
    return sum_list(nums) / len(nums)
