# Shelfy 技术架构

> 本文是当前实现的维护入口和代码定位索引。修改模块边界、执行入口、持久化模型、Visual 参数协议、UI 基础设施或前后端命令时，请同步更新本文。
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
主设置窗口关闭时只隐藏而不销毁或退出应用；托盘、Watcher 与 Scheduler 继续运行，用户可从托盘或 macOS Dock 重新打开窗口，只有托盘退出操作会结束进程。

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
- Settings 中的手动“模拟”是快速预览：每条规则的 walker 最多检查 500 个目录项，整份配置最多展示 10 个匹配后提前停止，并且无条件跳过 Shell 外部命令（即使 YAML 设置了 `run_in_simulation`）。CLI/MCP 的 simulate 仍保持全量兼容语义。
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

GUI 的手动运行走 `src-tauri/src/commands/orden.rs`，耗时工作交给 `src-tauri/src/orden_runtime.rs`。运行时维护最多 256 个任务状态，前端通过 `orden_task_status_cmd` 轮询，避免阻塞 Tauri command 线程。模拟请求会向 walker 下传扫描预算，因此会在读取目录期间提前停止，而不是先全量收集路径再截断结果。

Orden 配置名会规范化后同时作为 SQLite `orden_configs.name` 和 `<data_dir>/orden/<name>.yaml` 的文件名；支持 Unicode 字母与数字（包括中文）以及 `-`、`_`，并拒绝路径分隔符、`..` 和其他不安全字符。保存、读取、删除共用同一规范化逻辑，`.yaml` / `.yml` 扩展名不区分大小写。

MCP 操作指南由 `src-tauri/src/mcp.rs::help_text` 统一提供：`shelfy --mcp --help`、`shelfy --cli mcp --help` 打印指南后退出，Settings → MCP 的“操作指南”通过 `mcp_help_cmd` 展示同一内容。指南覆盖启动配置、读写工具边界、Orden 多规则模型、任务调度与先模拟后运行的安全流程。`shelfy_save_orden_job` 单独写入任务表；调度不嵌入规则 YAML。“每隔 1 小时”使用 `mode=interval, interval_minutes=60`，而 `0 * * * *` 表示每个整点。

## 自动化与调度

`OrdenJob` 绑定一个已保存的 Orden 配置，支持：

- `manual`：只允许显式运行。
- `fixed`：每日固定时间。
- `cron`：五字段 cron。
- `interval`：按上次成功执行时间间隔触发。
- `monitor`：watcher 发现路径匹配的新文件时触发。

`src-tauri/src/orden_jobs.rs` 负责到期判定、条件检查、并发去重和后台执行。`src-tauri/src/scheduler.rs` 负责常规定时整理、cron 与 keepalive；两者的调度事件写入 `scheduler_logs`。

自动运行可启用 `require_confirmation`。启用后，定时或 monitor 触发只以 simulate 模式完成规则扫描和动作预检，并把最新匹配数、错误数、日志与扫描时间保存到 `orden_jobs`；任务列表允许查看预检快照，用户通过确认 Dialog 手动执行时才以真实模式重新扫描并运行，成功后清空待确认状态。`extract` 的预检不是浅层模拟：ZIP 会读取全部条目、校验 CRC 并试配候选密码，7z/RAR 会执行 `7z t` 试配密码；预检不创建目标目录、不解压且不删除源包。

系统模板 `watch-extract-confirm` 提供该流程的可视化起点：规则 locations 是用户选择的源目录，extract `dest` 是新目录，`passwords` 是可维护且遮蔽显示的密码本，`delete_original=true` 仅在真实解压成功后生效；套用模板会同时创建 `monitor + require_confirmation` 自动运行。收到 `.7z.02/.002` 等任意后续卷时，extract 会按相同编号宽度定位 `.01/.001` 首卷，首卷缺失或卷号不连续时不删除任何卷。模板默认 `recursive=true, max_depth=3`，会继续处理本次解压新产生的内部 ZIP/7z/RAR，每层解压到内部压缩包所在目录的同名文件夹。递归不跟随符号链接，同一路径仅处理一次，总量最多 1000 个，层数限制为 1–10；达到限制时保留剩余压缩包并写 warn。修改规则源目录时，任务的 `watch_paths` 必须同步修改，二者当前不做隐式联动。

后台 LaunchAgent/keepalive 使用 `--autostart` 唤醒应用。单实例回调必须识别该参数并保持窗口隐藏，只有用户主动再次打开应用或点击界面入口时才显示 Settings。

macOS release 首次启动会由 `cli::ensure_cli_launcher` 创建 `~/.local/bin/shelfy` 包装器（自动附加 `--cli`），并幂等地为 zsh/bash 的 login/interactive 配置加入用户级 PATH；开发构建不修改用户 shell 配置。

## 应用更新

Settings → General → 软件更新提供手动检查、下载并安装，以及默认关闭的自动安装开关。`check_update_cmd` 只读取固定地址 `https://github.com/mcxen/shelfy/releases/latest/download/latest.json`；`install_update_cmd` 不接收前端提供的 URL，而是在后端重新检查清单。

macOS 应用内更新与 `Casks/shelfy.rb` 共同使用稳定端点 `Shelfy_universal-apple-darwin.app.zip`，版本化 ZIP 只用于发布追踪。下载阶段校验清单中的 SHA256 和字节数，staging 阶段校验 `Shelfy.app`、`cc.shelfy.app`、版本号、主可执行文件和执行位。独立 helper 再次限制 staging/目标路径并验证已安装 bundle，等待主进程退出后在同卷保留隐藏备份，再替换、ad-hoc 重签并重启；失败时恢复旧 bundle。不可写安装不会请求提权，Homebrew 安装交由 `brew upgrade --cask --greedy-latest mcxen/shelfy/shelfy`。Windows/Linux 当前可以检查版本，但自动替换仅支持 macOS。

`.github/workflows/build.yml` 在版本 tag 发布时同时上传版本化 ZIP、稳定名称 `Shelfy_universal-apple-darwin.app.zip` 和 `latest.json`。完整发布契约见 `../../docs/RELEASE_UPDATES.md`。

## 前端结构

```text
src/
  store/useAppStore.ts       # Tauri invoke 封装、共享状态与 Orden task 轮询
  components/Settings.tsx    # Settings 壳、导航、通用 Rules/History/General 协调
  components/settings/
    OrdenTab.tsx             # Orden 配置、编辑器、任务、模板、详情与执行结果
    OrdenVisualRuleCard.tsx  # 来源 → 条件 → 动作 的可视化规则
    OrdenPipelineEditor.tsx  # 条件/动作卡片轨道与参数检查器
    OrdenStepParameterEditor.tsx # 按 filter/action kind 绘制类型化参数并序列化 YAML value
    RulesTab.tsx             # SQLite 简单规则管理（含聚焦式新增/编辑 Dialog）
  components/ui/
    tag-input.tsx            # 标签、扩展名、MIME、密码候选等短值列表
    menu.tsx / select.tsx    # Portal 浮层及其主题化滚动容器
  index.css                  # 明暗 token、全局 scrollbar、surface 与基础排版
```

Settings 的 Orden 页面按需加载：首次进入 Advanced 时读取配置和任务；进入具体配置后才读取完整 YAML 与历史。窄窗口（小于 900px）使用可操作的卡片列表，桌面使用表格；不要新增只适用于表格的关键操作。

Settings 顶部不再提供独立 Ignore Tab；底层 `.shelfyignore` 读写能力保留用于兼容已有配置，但不占用主导航入口。

Orden 配置中心先按配置名与本地备注过滤，再统一按每页 6 条切片；桌面表格和窄屏卡片共用搜索词、页码和翻页控件。搜索词变化时回到第一页，增删导致页数减少时自动校正当前页。

Orden 运行历史列表只展示运行摘要；点击桌面行或窄屏卡片后打开详情 Dialog。详情解析该次 `logs_json`，按路径、消息、动作、级别搜索，并固定每页渲染 50 条，避免数百个文件日志同时挂载。基础 Rules 的新增/编辑同样使用聚焦 Dialog，主页面始终保留列表上下文。

Orden 编辑器一次只绑定一份配置：配置切换只能从配置中心发生；已有配置允许编辑名称，保存时执行真正的重命名并同步 YAML 文件、SQLite 配置、自动化任务与运行历史，不会复制出旧配置。新建草稿保存后转为已有配置。返回配置中心前会保护未保存修改，从编辑器发起的模拟/运行预览返回编辑器，从列表或详情发起的预览返回原视图。

“复制配置”只复制规则 YAML，并以 `<原名>-copy[-N]` 建立新的 `orden_configs` 记录和自增 ID；自动化任务与运行历史不复制，避免副本创建后立即继承调度或混淆审计记录。编辑器内复制会包含当前尚未保存的规则内容，复制完成后直接打开副本。

General 设置页位于 `components/settings/GeneralTab.tsx`，信息架构固定按“偏好设置 → 文件处理 → 自动化 → AI/MCP → 维护”排列。页面使用 `SettingGroup / SettingRow / SectionHeading` 形成紧凑表格式布局：说明列在左、控制列在右，Switch 共用右边线，关联输入和按钮在空间允许时同排。语言、主题、开机启动在桌面端保持一行三列；宽限期和文件占用归入文件处理；固定时间、Cron、后台保活和调度日志归入自动化；更新与配置导入导出归入维护。新增通用设置时先归入现有类别，不要直接在页面末尾追加新 Card。

Orden Visual 与 Source 的关系：Visual 覆盖当前 UI 可选的常用 filter/action，并序列化成 YAML；Source 是未知/高级语法的逃生入口。复杂 YAML 的未知字段与注释目前不能保证 Visual round-trip，切换前应视为可能丢失非可视化表达。

### Visual 参数编辑链路

```text
OrdenVisualRuleCard
  → OrdenPipelineEditor（步骤选择、排序、增删）
  → OrdenStepParameterEditor（按 mode + kind 选择字段 schema）
  → step.value YAML 片段
  → visualToOrdenYaml
  → Rust serde_yaml / build_filter / build_action
```

参数事实来源不是前端 preset，而是 Rust 工厂：

- 动作字段：`src-tauri/src/orden/actions/mod.rs::build_action`
- 过滤器字段：`src-tauri/src/orden/filters/mod.rs::build_filter`

Orden 面向用户统一使用“整理方案 → 分类规则 → 文件位置 → 匹配条件 → 处理方式”，调度统一称为“自动运行”。代码、SQLite 与 YAML 中的 config/rule/filter/action/job 字段保持不变，避免破坏兼容性；这些内部英文变量不得作为 Visual 界面文案直接显示。

Orden 动作与运行过程的展示统一通过 `src/lib/ordenI18n.ts` 映射 `workflow.steps`、`workflow.senders`、`workflow.levels`、`jobModes` 和 `runTriggers`；动作卡片、详情预览、Popup 快捷任务、任务列表、模拟结果与历史日志不得直接显示原始 action/sender/level/mode/trigger 标识。动作参数的枚举值使用 `workflow.params` 本地化。
- 前端字段 schema：`src/components/settings/OrdenStepParameterEditor.tsx`
- 默认示例：`src/components/settings/OrdenPipelineEditor.tsx::PRESETS`

新增或修改 filter/action 参数时，必须同步核对这四处。类型化编辑器当前覆盖移动、复制、重命名、解压、压缩、链接、写文件、日志、Shell、废纸篓和永久删除等全部可选动作。解压使用密码候选列表 `passwords`，可用 `recursive + max_depth` 有界处理内部压缩包；压缩使用单密码 `password`。密码字段、候选标签及 7-Zip 运行日志按用户要求直接显示明文。

`archivefile` 过滤器及 `extract format: auto` 通过文件头识别 ZIP、7z、RAR，不依赖普通文件扩展名；分卷 7z 仍通过卷号命名定位首卷，再读取首卷文件头。递归解压发现内部压缩包时复用相同的内容识别。`compress format: auto` 从渲染后的目标文件名选择 7z/RAR，否则回退 ZIP。

短值列表统一走 `TagInput`，底层仍保存原有逗号字符串或 YAML sequence。适用范围包括配置 tags/skip-tags、规则 tags、任务 tags、extension、MIME 和其他短枚举列表。路径、命令和自由文本不要为了复用 TagInput 而强行拆分。

### UI 基础设施与滚动条

`docs/UI_SPEC.md` 是视觉与交互规范，`src/index.css` 是 token 和全局实现。明暗主题均通过 CSS variables 驱动，业务组件不得硬编码主题颜色。

所有可滚动产品 surface 使用全局 8px 主题化滚动条：

- track/corner/track-piece 透明；
- thumb 使用 `--scrollbar-thumb`，hover 使用 `--scrollbar-thumb-hover`，active 使用 `--scrollbar-thumb-active`；
- `:root` 的 `color-scheme` 随 light/dark 切换，避免 WebView 使用错误的系统 scrollbar；
- Portal 内的 Menu/Select 滚动容器显式使用 `bg-popover`，透明轨道不会露出浅色宿主背景；
- `.no-scrollbar` 只允许用于顶部紧凑导航与横向步骤轨道，且仍需支持滚轮、触控板与键盘滚动。

修改滚动行为时优先改 `src/index.css` 或 `src/components/ui/` 基础组件，不要在单个业务页面复制 `::-webkit-scrollbar` 规则。

Settings 与 Popup 都使用无传统标题栏的自定义窗口；顶部中央必须保留可见的 `GripHorizontal` 抓手并标记 `data-tauri-drag-region`，标题区空白仍可拖动，导航和关闭按钮不得被拖拽层遮挡。

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
- `orden_configs`：YAML 的索引/缓存，`id INTEGER PRIMARY KEY AUTOINCREMENT` 是内部稳定身份，磁盘 YAML 仍是配置事实来源且不嵌入 ID。重命名保留 ID；复制生成新 ID；GUI/MCP 保存或磁盘同步手写 YAML 时由后端自动分配 ID。
- `orden_jobs`：自动化任务。
- `orden_run_logs`：Orden 的运行结果和结构化日志 JSON。
- `scheduler_logs`：调度、保活与失败事件。
- `settings`：语言、权限相关选项、自动更新、MCP、cron、keepalive 等。

## History 与撤销边界

现有 Undo 模型是“从 destination rename 回 source”，因此只适用于 move/rename。

- Orden `move` / `rename`：真实执行成功后写 `action_logs`，可在主 History 中 Undo。
- Orden `copy` / `delete` / `trash` / `write` / `shell` / links / archive：写入 `orden_run_logs` 供审计，但不写可撤销 History。
- 新动作若要接入主 History，必须先定义其 `reversible` 语义和恢复策略，不能复用 move 的反向 rename。

## MCP 安全边界

MCP 是本地 stdio JSON-RPC 服务，启动方式为 `shelfy --mcp` 或 `shelfy --cli mcp`。

- `mcp_enabled=false` 时不暴露 tools/resources。
- 默认仅提供读取和 simulate；`mcp_allow_write=true` 才暴露 `shelfy_save_orden_config`、`shelfy_save_orden_job`、扫描文件夹和实际 Orden run。MCP 创建配置只提交 `name + yaml`，内部 ID 由 Shelfy 管理；任务调度通过结构化 job schema 写入 SQLite。
- stdio 模式不监听网络端口；Settings 中的 HTTP 字段为未来 bridge 预留。

实现位于 `src-tauri/src/mcp.rs`，命令与资源命名为 `shelfy_*` 和 `shelfy://orden/...`。

## 维护清单

修改时同步检查：

1. 新增 Orden filter/action：更新 `RESEARCH.md`、`TASK.md`，并补核心单元测试。
2. 新增 Tauri command：更新 `src-tauri/src/lib.rs` 的 handler、`useAppStore.ts` 封装和本文调用入口表。
3. 修改持久化：更新 `src-tauri/src/db/` 迁移、本文存储清单与导入导出逻辑。
4. 修改 Undo 行为：先更新“History 与撤销边界”，再改日志模型和 UI。
5. 修改 UI 响应式策略：更新 `docs/UI_AUDIT.md` 与 `docs/ORDEN_FLOW_DESIGN.md`。
6. 修改 Visual 参数：同步 Rust 工厂键、`OrdenStepParameterEditor` schema、`PRESETS` 和本文“Visual 参数编辑链路”。
7. 修改全局 UI token、Portal 或滚动条：核对 `docs/UI_SPEC.md`、`src/index.css` 与 `src/components/ui/`，并验证浅色/深色。
8. 完成后至少运行：前端 `npm run build` 与 `git diff --check`；涉及 Rust 时在 `src-tauri/` 运行 `cargo build` 和相关 `cargo test`。

## 关联文档

- `../../TASK.md`：Orden 内核进度、待办和当前阻塞。
- `../../RESEARCH.md`：organize-tool 行为对照、过滤器/动作语义和技术决策。
- `../../AGENTS.md`：代码与验证约定。
- `../../docs/ORDEN_FLOW_DESIGN.md`：Visual 编辑器与模板中心的交互协议。
- `../../docs/UI_AUDIT.md`：响应式、密度和可访问性审计项。
