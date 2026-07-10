# AGENTS

> 本文件给 opencode / 后续 agent 提供项目工作约定。修改 `orden` 模块或推进任务前，先读本文件 + `TASK.md` + `RESEARCH.md`。

## 项目背景

Shelfy 是 Tauri (Rust + React) 桌面文件整理工具，当前后端 `src-tauri/src/` 有简化规则引擎（DB 存扩展名+正则+目标）。正在用 Rust 复刻 [organize-tool](https://github.com/tfeldmann/organize) v3 的核心能力，集成进 `src-tauri/src/orden/`，之后再独立成 crate。项目内部名称使用 `orden`，文档中提到 organize-tool 时指上游 Python 项目。

## 关键文档（必须维护）

- `RESEARCH.md` — organize-tool 源码调研、架构分析、过滤器/动作清单、Rust crate 复用映射、风险决策。**新增过滤器/动作或遇到 organize 行为疑问时，先查这里。**
- `TASK.md` — Orden 内核技术方案、目录结构、任务拆解与进度勾选。**每完成/开始一个子任务，更新对应勾选框和"当前阻塞/下一步"。**
- `AGENTS.md`（本文件）— 工作约定。

## 工作流

1. **开工前**：读 `TASK.md` 的"当前阻塞"和"下一步"；读 `RESEARCH.md` 对应章节确认 organize 原始行为
2. **实现时**：优先复用 `Cargo.toml` 已列 crate（globset/regex/sha1/sha2/md-5/hex/mime_guess/trash/serde_yaml/chrono/once_cell/anyhow），避免重复造轮子；模板引擎与 organize 语法绑定处除外（见 RESEARCH.md §7）
3. **完成后**：勾选 `TASK.md` 进度；若发现 organize 行为与调研记录不符或新增功能，补 `RESEARCH.md`
4. **遇阻**：在 `TASK.md` "当前阻塞"记录，"下一步"写明解法

## 编码约定

- 模仿现有 `src-tauri/src/` 代码风格（`use` 顺序、错误用 `Result<_, String>` 与现有模块一致、中文注释只在必要处）
- 过滤器实现 `orden::Filter` trait，动作实现 `orden::Action` trait，命名小写 snake_case 文件
- YAML 配置解析在 `mod.rs` 用 `serde_yaml`，简写形式（str/dict/value）用自定义 deserialize
- 不写无意义注释；不主动加 emoji
- **不提交任何改动除非用户明确要求**（git commit/push 须用户确认）

## 验证

- 改动后跑 `cargo build`（workdir=`src-tauri`）；有测试跑 `cargo test`
- 模板/size/duplicate 等核心逻辑必须有单元测试

## 集成约束

- `orden` 模块与现有 `rules.rs` **并存**：DB 简单规则继续供 UI 管理，YAML 高级规则走 `orden` 引擎
- `orden` 的 `move` / `rename` 动作执行成功后写入 Shelfy History；其他动作在定义 Undo 语义前不要写入可撤销 move 日志
- CLI 在 `cli.rs` 支持 `orden|organize sim|run|check <config>` 子命令，不破坏现有 `--cli` 子命令
- Tauri commands 暴露 `orden_list/load/save/delete/check/run`，Settings 的 Advanced 面板是当前 GUI 入口

## 调研源码位置

organize-tool 源码快照（已 clone，可能被系统清理）：
`/var/folders/ng/zj6dzdcj0vx8chjb0yp1t4740000gn/T/opencode/organize-src`

若被清理：`git clone --depth 1 https://github.com/tfeldmann/organize.git` 重新获取，或查 `RESEARCH.md`（已记录全部关键源码内容）。
