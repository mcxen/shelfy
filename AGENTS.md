# AGENTS

> 本文件给 opencode / 后续 agent 提供项目工作约定。先用本文的“快速定位”确定最小阅读范围，不要无差别读取全部长文档。

## 项目背景

Shelfy 是 Tauri (Rust + React) 桌面文件整理工具，当前后端 `src-tauri/src/` 有简化规则引擎（DB 存扩展名+正则+目标）。正在用 Rust 复刻 [organize-tool](https://github.com/tfeldmann/organize) v3 的核心能力，集成进 `src-tauri/src/orden/`，之后再独立成 crate。项目内部名称使用 `orden`，文档中提到 organize-tool 时指上游 Python 项目。

## 关键文档（必须维护）

- `public/architecture/README.md` — **代码与运行逻辑的首要索引**：模块边界、调用链、存储、前端结构、Visual 参数编辑与 UI 基础设施。开始定位代码时先读对应章节。
- `RESEARCH.md` — organize-tool 源码调研、架构分析、过滤器/动作清单、Rust crate 复用映射、风险决策。**新增过滤器/动作或遇到 organize 行为疑问时，先查这里。**
- `TASK.md` — Orden 内核技术方案、目录结构、任务拆解与进度勾选。**每完成/开始一个子任务，更新对应勾选框和"当前阻塞/下一步"。**
- `docs/UI_SPEC.md` — UI token、组件、滚动条、布局与交互的规范来源。修改视觉或交互前先查对应条目。
- `AGENTS.md`（本文件）— 工作约定。

## 快速定位（节省 token）

默认只读 `AGENTS.md` 和 `public/architecture/README.md` 的相关章节；仅在下列情况追加读取长文档：

| 任务 | 先读 | 主要代码入口 |
| --- | --- | --- |
| Orden 执行、YAML、过滤器、动作 | Architecture“两套规则引擎 / Orden 执行路径” | `src-tauri/src/orden/mod.rs`、`orden/filters/mod.rs`、`orden/actions/mod.rs` |
| 上游 organize 行为兼容 | `RESEARCH.md` 对应过滤器/动作章节 | `/var/.../organize-src` 快照或上游仓库 |
| GUI Visual 编辑器 | Architecture“前端结构 / Visual 参数编辑” + `docs/UI_SPEC.md` | `OrdenTab.tsx` → `OrdenVisualRuleCard.tsx` → `OrdenPipelineEditor.tsx` → `OrdenStepParameterEditor.tsx` |
| Tag/短值输入 | Architecture“Visual 参数编辑” | `src/components/ui/tag-input.tsx` |
| 菜单、Select、Dialog、滚动条、明暗主题 | `docs/UI_SPEC.md` Interaction/Visual Tokens | `src/index.css`、`src/components/ui/` |
| Tauri command / store 调用 | Architecture“Orden 的调用入口” | `src-tauri/src/commands/`、`src-tauri/src/lib.rs`、`src/store/useAppStore.ts` |
| 调度、监控、后台任务 | Architecture“自动化与调度” | `orden_jobs.rs`、`scheduler.rs`、`watcher.rs` |
| History / Undo | Architecture“History 与撤销边界” | `db/logs.rs`、`rules.rs`、Orden move/rename |

使用 `rg` 定位具体符号和文件；不要为了一个局部 UI 或动作问题全文读取 `TASK.md`、`RESEARCH.md` 或整个大组件。

## 工作流

1. **开工前**：按“快速定位”读取 Architecture 对应章节；推进既有里程碑时再读 `TASK.md` 的“当前阻塞/下一步”，涉及 organize 语义时才读 `RESEARCH.md` 对应章节
2. **实现时**：优先复用 `Cargo.toml` 已列 crate（globset/regex/sha1/sha2/md-5/hex/mime_guess/trash/serde_yaml/chrono/once_cell/anyhow），避免重复造轮子；模板引擎与 organize 语法绑定处除外（见 RESEARCH.md §7）
3. **完成后**：勾选 `TASK.md` 进度；若发现 organize 行为与调研记录不符或新增功能，补 `RESEARCH.md`
4. **遇阻**：在 `TASK.md` "当前阻塞"记录，"下一步"写明解法

## 编码约定

- 模仿现有 `src-tauri/src/` 代码风格（`use` 顺序、错误用 `Result<_, String>` 与现有模块一致、中文注释只在必要处）
- 过滤器实现 `orden::Filter` trait，动作实现 `orden::Action` trait，命名小写 snake_case 文件
- YAML 配置解析在 `mod.rs` 用 `serde_yaml`，简写形式（str/dict/value）用自定义 deserialize
- 不写无意义注释；不主动加 emoji
- **不提交任何改动除非用户明确要求**（git commit/push 须用户确认）

## 前端与 UI 约定

- UI 规范以 `docs/UI_SPEC.md` 为准，组件优先复用 `src/components/ui/` 的 coss/shadcn 层，不直接使用原生风格控件或另造视觉体系。
- 全局颜色、surface、focus、滚动条必须使用 `src/index.css` token；不得在业务组件硬编码明暗颜色。
- 所有可滚动产品区域使用 `src/index.css` 的主题化细滚动条：透明 track、muted thumb、primary hover、ring active。仅紧凑横向导航和卡片轨道可用 `.no-scrollbar` 隐藏滑块但必须保留滚动能力。
- Portal 浮层的滚动容器必须有与浮层一致的 token 背景。菜单见 `ui/menu.tsx`，Select 见 `ui/select.tsx`，避免 macOS 深色模式露出系统白色轨道。
- Orden Visual 是结构化编辑器：当前可选动作必须在 `OrdenStepParameterEditor.tsx` 提供对应字段组件；不能退回用 YAML textarea 代替常用参数。
- 解压动作必须暴露 `dest/format/passwords/delete_original/on_conflict/rename_template/autodetect_folder`；压缩动作必须暴露单密码输入与显隐控制。
- 标签、跳过标签、扩展名、MIME 等短值列表使用 `ui/tag-input.tsx`，支持回车/逗号添加和逐项删除；密码列表使用遮蔽显示，辅助功能文本不得泄露密码。
- 修改 Visual 参数字段时同时核对 Rust `orden/actions/mod.rs` 或 `orden/filters/mod.rs` 的解析键，并保证序列化 YAML 仍可被后端读取。

## 验证

- 改动后跑 `cargo build`（workdir=`src-tauri`）；有测试跑 `cargo test`
- 仅前端改动至少跑 `npm run build` 和 `git diff --check`
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
