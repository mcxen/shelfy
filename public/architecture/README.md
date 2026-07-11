# Shelfy 技术架构

> 本文是当前实现的维护入口。修改模块边界、执行入口、持久化模型或前后端命令时，请同步更新本文。
> Orden 的上游行为调研见仓库根目录 `RESEARCH.md`，内核任务状态见 `TASK.md`。

## 系统概览

```text
React / Tauri WebView
  ├─ Popup：待处理文件与快速操作
  └─ Settings：Rules、Orden、模板、历史、通用设置
          │ invoke / event
          ▼
Rust Tauri commands
  ├─ 简单规则 Rules
  ├─ 高级规则 Orden
  ├─ Scheduler / watcher / tray
  └─ MCP stdio 服务
          │
          ├─ SQLite（设置、Rules、History、Orden 元数据与运行记录）
          └─ App data dir（Orden YAML 与模板 YAML）
```

入口与装配在 `src-tauri/src/lib.rs`：启动时初始化数据库、托盘、文件监控和调度器，并集中注册 Tauri commands。

## 两套规则引擎

| 维度 | Rules | Orden |
| --- | --- | --- |
| 产品定位 | 轻量、实时文件整理 | 高级、可编排文件自动化 |
| 配置来源 | SQLite | YAML 文件 + SQLite 索引 |
| 主要入口 | watcher、手动扫描 | GUI、CLI、任务调度、监控、MCP |
| 匹配能力 | 扩展名 + 单个正则 | locations、过滤器管线、tags、files/dirs |
| 动作能力 | move / copy / ignore | move、copy、rename、trash、archive、shell 等 |
| 历史/撤销 | 写入 `action_logs`，可撤销 move/rename | move/rename 写入可撤销 History；其他动作当前只进入 Orden 运行审计 |

不要把两者合并：Rules 保持 watcher 的低延迟、简单模型；Orden 保持 YAML 兼容与多步骤语义。两者共用设置、历史界面和文件权限提示，但不共用执行器。

## Rules 执行路径

```text
notify watcher
  → FolderWatcher pending queue（grace period / lock / ignore）
  → rules::process_file
  → 按 priority 找到首个启用且匹配的 SQLite Rule
  → move / copy / ignore
  → action_logs + 前端 file-organized event + 系统通知
```

- `src-tauri/src/watcher.rs`：监听目录、silent/manual/paused 模式、待处理队列和通知。
- `src-tauri/src/rules.rs`：文件安全检查、`.shelfyignore`、规则匹配、跨设备移动和 `action_logs` 写入。
- `src-tauri/src/db/rules.rs`：Rules 与 watched folders 的持久化。

Watcher 对新文件也会调用 `orden_jobs::run_monitor_jobs`，但这仅触发启用了 `monitor` 模式的 Orden 任务；它不会把所有 watcher 文件交给全部 Orden 配置。

## Orden 执行路径

```text
YAML config
  → Config::from_string
  → Rule[]
      location walker
      → filter pipeline (all / any / none)
      → action pipeline（按顺序）
  → RunResult { success, errors, simulate, logs }
```

`src-tauri/src/orden/mod.rs` 是内核入口；各能力按以下目录拆分：

```text
orden/
  filters/       # extension、regex、size、duplicate、exif 等
  actions/       # move、copy、rename、trash、archive、shell 等
  walker.rs      # location 遍历、深度、排除
  template.rs    # organize 风格 {variable.method()} 模板
  conflict.rs    # skip / overwrite / rename_new 等冲突策略
  resource.rs    # 单个处理对象与跨步骤变量
```

执行规则：

- `simulate=true` 只产生日志，不执行文件变更，也不写可撤销 History。
- 一条规则的 actions 顺序执行；move 后的后续动作使用新的资源路径。
- `copy` 可扇出到多个目标，`move` 保持单一目标，避免首个移动后源文件消失。
- 模板和过滤器可以写入/读取 `Resource.vars`。

## Orden 的调用入口

| 调用方 | 入口 | 执行方式 | 审计记录 |
| --- | --- | --- | --- |
| Settings | `orden_run_cmd` | `orden_runtime::spawn` 后前端轮询 task status | `orden_run_logs` |
| CLI | `shelfy --cli orden sim/run/check` | 同步 CLI 流程 | 输出到终端 |
| 自动化任务 | `orden_jobs::run_due_jobs` | 每个任务独立线程 | `orden_run_logs` + `scheduler_logs` |
| watcher monitor | `orden_jobs::run_monitor_jobs` | 同自动化任务 | 同上 |
| MCP | `shelfy_orden_simulate/run` | MCP worker thread 后 join | `orden_run_logs` |

GUI 的手动运行走 `src-tauri/src/commands/orden.rs`，耗时工作交给 `src-tauri/src/orden_runtime.rs`。运行时维护最多 256 个任务状态，前端通过 `orden_task_status_cmd` 轮询，避免阻塞 Tauri command 线程。

## 自动化与调度

`OrdenJob` 绑定一个已保存的 Orden 配置，支持：

- `manual`：只允许显式运行。
- `fixed`：每日固定时间。
- `cron`：五字段 cron。
- `interval`：按上次成功执行时间间隔触发。
- `monitor`：watcher 发现路径匹配的新文件时触发。

`src-tauri/src/orden_jobs.rs` 负责到期判定、条件检查、并发去重和后台执行。`src-tauri/src/scheduler.rs` 负责常规定时整理、cron 与 keepalive；两者的调度事件写入 `scheduler_logs`。

## 前端结构

```text
src/
  store/useAppStore.ts       # Tauri invoke 封装、共享状态与 Orden task 轮询
  components/Settings.tsx    # Settings 壳、导航、通用 Rules/History/General 协调
  components/settings/
    OrdenTab.tsx             # Orden 配置、编辑器、任务、模板、详情与执行结果
    OrdenVisualRuleCard.tsx  # 来源 → 条件 → 动作 的可视化规则
    OrdenPipelineEditor.tsx  # 条件/动作卡片轨道与参数检查器
    RulesTab.tsx             # SQLite 简单规则管理
```

Settings 的 Orden 页面按需加载：首次进入 Advanced 时读取配置和任务；进入具体配置后才读取完整 YAML 与历史。窄窗口（小于 900px）使用可操作的卡片列表，桌面使用表格；不要新增只适用于表格的关键操作。

Orden Visual 与 Source 的关系：Visual 仅覆盖常见字段，并序列化成 YAML；Source 是完整能力入口。复杂 YAML 的未知字段与注释目前不能保证 Visual round-trip，切换前应视为可能丢失非可视化表达。

## 数据与文件存储

应用数据根目录由 `directories::ProjectDirs("cc", "shelfy", "shelfy")` 决定：

```text
<data_dir>/
  shelfy.db
  orden/
    <config>.yaml
    templates/
      <system-template>.yaml
      <custom-template>.yaml
```

SQLite 主要数据：

- `rules`、`watched_folders`：简单规则与监控目录。
- `action_logs`：基础 Rules 与 Orden move/rename 的可撤销历史。
- `orden_configs`：YAML 的索引/缓存，磁盘 YAML 仍是配置事实来源。
- `orden_jobs`：自动化任务。
- `orden_run_logs`：Orden 的运行结果和结构化日志 JSON。
- `scheduler_logs`：调度、保活与失败事件。
- `settings`：语言、权限相关选项、MCP、cron、keepalive 等。

## History 与撤销边界

现有 Undo 模型是“从 destination rename 回 source”，因此只适用于 move/rename。

- Orden `move` / `rename`：真实执行成功后写 `action_logs`，可在主 History 中 Undo。
- Orden `copy` / `delete` / `trash` / `write` / `shell` / links / archive：写入 `orden_run_logs` 供审计，但不写可撤销 History。
- 新动作若要接入主 History，必须先定义其 `reversible` 语义和恢复策略，不能复用 move 的反向 rename。

## MCP 安全边界

MCP 是本地 stdio JSON-RPC 服务，启动方式为 `shelfy --mcp` 或 `shelfy --cli mcp`。

- `mcp_enabled=false` 时不暴露 tools/resources。
- 默认仅提供读取和 simulate；`mcp_allow_write=true` 才暴露扫描文件夹和实际 Orden run。
- stdio 模式不监听网络端口；Settings 中的 HTTP 字段为未来 bridge 预留。

实现位于 `src-tauri/src/mcp.rs`，命令与资源命名为 `shelfy_*` 和 `shelfy://orden/...`。

## 维护清单

修改时同步检查：

1. 新增 Orden filter/action：更新 `RESEARCH.md`、`TASK.md`，并补核心单元测试。
2. 新增 Tauri command：更新 `src-tauri/src/lib.rs` 的 handler、`useAppStore.ts` 封装和本文调用入口表。
3. 修改持久化：更新 `src-tauri/src/db/` 迁移、本文存储清单与导入导出逻辑。
4. 修改 Undo 行为：先更新“History 与撤销边界”，再改日志模型和 UI。
5. 修改 UI 响应式策略：更新 `docs/UI_AUDIT.md` 与 `docs/ORDEN_FLOW_DESIGN.md`。
6. 完成后至少运行：前端 `npm run build`；涉及 Rust 时在 `src-tauri/` 运行 `cargo build` 和相关 `cargo test`。

## 关联文档

- `../../TASK.md`：Orden 内核进度、待办和当前阻塞。
- `../../RESEARCH.md`：organize-tool 行为对照、过滤器/动作语义和技术决策。
- `../../AGENTS.md`：代码与验证约定。
- `../../docs/ORDEN_FLOW_DESIGN.md`：Visual 编辑器与模板中心的交互协议。
- `../../docs/UI_AUDIT.md`：响应式、密度和可访问性审计项。
