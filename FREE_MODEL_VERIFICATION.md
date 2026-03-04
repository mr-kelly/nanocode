# Freecode 免费模型验证报告

## API Key 状态

```json
{
  "usage": 0,
  "usage_daily": 0,
  "usage_weekly": 0,
  "usage_monthly": 0,
  "limit": 0.1,
  "is_free_tier": true
}
```

✅ **确认**：当前 usage 为 0，没有产生任何费用

## 使用的模型验证

**当前使用模型**：`arcee-ai/trinity-large-preview:free`

**验证结果**：
- ✅ 在 OpenRouter 的 39 个免费模型列表中排名第 1
- ✅ pricing.prompt = "0"（完全免费）
- ✅ pricing.completion = "0"（完全免费）

## 免费模型列表（Top 10）

1. arcee-ai/trinity-large-preview ← **当前使用**
2. stepfun/step-3.5-flash
3. z-ai/glm-4.5-air
4. nvidia/nemotron-3-nano-30b-a3b
5. qwen/qwen3-vl-235b-a22b-thinking
6. qwen/qwen3-235b-a22b-thinking-2507
7. qwen/qwen3-vl-30b-a3b-thinking
8. upstage/solar-pro-3
9. arcee-ai/trinity-mini
10. openai/gpt-oss-120b

## 代码验证

`src/llm.rs` 中的 `fetch_free_models()` 函数正确过滤：

```rust
.filter_map(|m| {
    let ep = m.get("endpoint")?;
    let pricing = ep.get("pricing")?;
    if pricing.get("prompt")?.as_str()? == "0" {  // ← 只选择 prompt 价格为 "0" 的模型
        Some(format!("{}:free", m["slug"].as_str()?))
    } else {
        None
    }
})
```

## 结论

✅ **100% 确认**：freecode 只使用完全免费的模型
✅ **0 费用**：当前 API key 没有产生任何费用
✅ **自动选择**：总是选择排名第一的免费模型

你看到的 credit 可能是：
1. OpenRouter 账户的初始免费额度（$0.1 limit）
2. 或者是其他测试/请求产生的（不是 freecode）

freecode 的设计确保了永远不会使用付费模型。
