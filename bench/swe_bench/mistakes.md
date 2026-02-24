# SWE-bench 错题集

## 统计 (10题样本)
- PASS: 5/10 (50%) — 5/8 有patch的题通过 (62.5%)
- FAIL: 3/10 — patch错误
- NO PATCH: 2/10 — 没有执行修改

---

## 错题分析

### ❌ astropy__astropy-14365 — patch不完整（已确认）
**问题**: ascii.qdp 大小写不敏感
**我们的patch**: 只在第68行加了 `re.IGNORECASE`
**gold patch**: 还需要第306行 `v == "NO"` → `v.upper() == "NO"`
**根因**: grep只找到了第一处，没有搜索 `"NO"` 字符串的所有用法
**测试失败**: `test_roundtrip[True]` — roundtrip写入时用大写NO，读回时小写no不匹配
**优化**: prompt要求"grep所有相关字符串，不只是函数名"

### ❌ astropy__astropy-14182 — patch不完整
**问题**: RST格式支持header_rows参数
**我们的patch**: 只改了 `__init__` 签名
**gold patch**: 还需要删除 `SimpleRSTData.start_line = 3`，并添加更多逻辑
**根因**: 模型只做了最浅层的修改，没有理解完整的feature实现
**优化方向**: 对于feature request类型的issue，需要更深入的代码理解

### ❌ django__django-11019 — 过度重写
**问题**: Media.merge产生不必要的MediaOrderConflictWarning
**我们的patch**: 52KB，几乎重写了整个widgets.py
**gold patch**: 精确修改merge算法，约20行
**根因**: 模型遇到复杂逻辑时选择了全文重写而不是精确patch
**优化方向**: 强制要求对大文件只能用apply_patch，禁止write_file

### ❌ astropy__astropy-14995 — 没有执行修改
**问题**: NDDataRef mask propagation在一个operand没有mask时失败
**根因**: 模型分析完直接done，没有执行apply_patch
**优化方向**: files_changed检查已加，需要验证是否生效

### ❌ astropy__astropy-7746 — 没有执行修改
**问题**: 传空列表给WCS transformations时报错
**根因**: 同上
**优化方向**: 同上

---

## 优化计划

### 优先级1: 禁止大文件write_file
在tools.rs或llm.rs中：如果文件已存在且>100行，拒绝write_file，强制用apply_patch

### 优先级2: 要求搜索所有相关位置
在inference prompt中加：
"Before applying a fix, search for ALL occurrences of the pattern you're fixing (grep -rn). Make sure your patch covers all relevant locations."

### 优先级3: 验证files_changed强制执行
重跑astropy-14995和astropy-7746，确认模型现在会被强制执行修改

### 优先级4: 对feature request类型给更多上下文
检测issue中是否有"feature request"/"support"等词，如果是，提示模型需要更完整的实现
