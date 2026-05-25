# TODO — qtcloud-code-cli

## P0 已完成 ✅

- [x] CLI 命令框架（`scan` / `list-rules`）
- [x] tree-sitter-rust 集成，Rust 代码解析
- [x] 递归扫描目录下所有 `.rs` 文件
- [x] 检测器：过长函数（>30 行）
- [x] 检测器：宽泛 unsafe 块（>5 条语句）
- [x] 输出格式：JSON（`--format json`）和终端文本
- [x] 错误处理：解析失败、权限拒绝时跳过
- [x] 集成测试：使用 `examples/default` 作为 fixture

## 高优

- [x] 检测器：未使用变量（走 rustc 路径，运行 `cargo check --message-format=json` 解析）
- [x] file_path 输出改为绝对路径（入口处 canonicalize）
- [x] 多语言：`LanguageParser` trait 提取后，加 Python 支持

## 中优

- [x] tree-sitter-go 绑定（含 `function_declaration` / `method_declaration` 节点支持）
- [x] tree-sitter-dart 绑定（`function_signature` 嵌套结构处理）
- [x] tree-sitter-typescript 绑定（含 `.ts` / `.tsx` 双解析器）
- [x] 通用检测器：过长函数（跨语言，同时支持 Rust `function_item` 和 Python `function_definition`）
- [x] 通用检测器：过长参数列表（跨语言，使用 `child_by_field_name("parameters")`）
- [x] `--rules` 选择启用的检测器（`--rules long-function,long-parameter-list`）
- [x] `.quanttide/code/contract.yaml` 配置文件（从扫描目录向上查找，支持 `code.rules` 字段）

