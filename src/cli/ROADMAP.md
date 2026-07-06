# ROADMAP — qtcloud-code-cli

## 当前

- [ ] `ListRules` 与 `contract list` 重复 — 删掉 `ListRules`，统一走 `contract list`
     当前 patch 先废弃（`#[deprecated]` 提示走 `contract list`），下一个 minor 移除
