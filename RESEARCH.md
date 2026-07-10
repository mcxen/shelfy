# RESEARCH - organize-tool 源码调研

> 目标：用 Rust 在 Shelfy 中复刻 [organize-tool](https://github.com/tfeldmann/organize) (v3) 的核心功能，集成进 `src-tauri/src/orden/`，之后再独立成 crate。
> 调研源码快照：`/var/folders/ng/zj6dzdcj0vx8chjb0yp1t4740000gn/T/opencode/organize-src`（已 git clone）。

## 1. organize-tool 架构总览

Python 实现，基于 pydantic + jinja2 + yaml。核心管线：

```
Config (yaml)
 └─ Rule[]
     ├─ locations: Location[]          # 遍历起点
     ├─ filters: Filter[]              # 过滤管道（all/any/none 模式）
     ├─ actions: Action[]              # 顺序执行
     └─ targets: "files" | "dirs"
```

执行流程（`rule.py:Rule.execute`）：
1. 对每个 location，用 `Walker` 遍历文件/目录 → 产出 `Resource`
2. `filter_pipeline` 按 `filter_mode`（all/any/none）跑过滤器；分别对应 AND / OR / NONE，过滤器可向 `res.vars` 写变量
3. 匹配则顺序跑 `action_pipeline`；动作可改 `res.path`（move 后续动作作用于新路径）
4. `res.walker_skip_pathes` 中的路径在剩余遍历中跳过（move 后避免重复处理）
5. `simulate=true` 时只打印不执行

`Resource`（`resource.py`）：每个文件一个，携带 `path` / `basedir` / `vars`（过滤器/动作间共享的动态字典）/ `walker_skip_pathes`。

模板（`template.py`）：jinja2 环境，分隔符 `{` `}`，`StrictUndefined`，支持 `{var}`、`{var.key}`、`{var.method()}`、`{created.strftime('%Y-%m')}`，渲染后 `expanduser` + `expandvars`。

注册表（`registry.py`）：`FILTERS` / `ACTIONS` 字典，按名字小写查找；`filters.ALL` / `actions.ALL` 元组注册全部。

## 2. 过滤器清单（`organize/filters/`）

| 过滤器 | 文件 | 功能 | 返回变量 | files | dirs |
|--------|------|------|----------|-------|------|
| `extension` | extension.py | 按扩展名匹配（可多值，去点小写） | `{extension}` | ✓ | ✗ |
| `name` | name.py | startswith/contains/endswith + simplematch 通配 | `{name}` | ✓ | ✓ |
| `regex` | regex.py | 正则匹配文件名，命名组 `(?P<x>)` 返回 | `{regex.x}` | ✓ | ✓ |
| `size` | size.py | 大小约束 `">= 500 MB"`，多约束逗号分隔 | `{size.bytes/traditional/binary/decimal}` | ✓ | ✓ |
| `created` | created.py | 创建时间（Win=ctime，Unix=birthtime，回退 stat） | `{created}` datetime | ✓ | ✓ |
| `lastmodified` | lastmodified.py | 修改时间 mtime | `{lastmodified}` | ✓ | ✓ |
| `date_added` | date_added.py | macOS 专属（mdls kMDItemDateAdded） | `{date_added}` | ✓ | ✓ |
| `date_lastused` | date_lastused.py | macOS 专属（mdls kMDItemLastUsedDate） | `{date_lastused}` | ✓ | ✓ |
| `empty` | empty.py | 空文件/空目录 | - | ✓ | ✓ |
| `duplicate` | duplicate.py | 快速重复检测：size→前1KB chunk hash→全文件 hash | `{duplicate.original}` | ✓ | ✗ |
| `hash` | hash.py | 计算文件 hash（hashlib，默认 md5） | `{hash}` | ✓ | ✗ |
| `mimetype` | mimetype.py | 按 MIME 类型（mimetypes.guess_type，前缀匹配 "audio"） | `{mimetype}` | ✓ | ✗ |
| `filecontent` | filecontent.py | 文件内容正则；支持 .txt/.md/.log/.pdf(pdftotext/pdfminer)/.docx(docx2txt) | `{filecontent.group}` | ✓ | ✗ |
| `exif` | exif.py | EXIF 过滤（exifread 或 exiftool），按 dotted key + glob 值匹配 | `{exif.*}` | ✓ | ✗ |
| `python` | python.py | 执行 python 代码过滤（exec） | `{python.*}` | ✓ | ✓ |

时间过滤器公共基类 `TimeFilter`：years/months/weeks/days/hours/minutes/seconds + mode(older/newer) + timezone，用 `arrow.now().shift(...)` 算比较时间。

## 3. 动作清单（`organize/actions/`）

| 动作 | 文件 | 功能 | standalone | files | dirs |
|------|------|------|------------|-------|------|
| `move` | move.py | 移动，dest 模板，冲突解决 | ✗ | ✓ | ✓ |
| `copy` | copy.py | 复制，continue_with=copy/original | ✗ | ✓ | ✓ |
| `rename` | rename.py | 同目录重命名（禁含斜杠） | ✗ | ✓ | ✓ |
| `delete` | delete.py | 永久删除（无恢复） | ✗ | ✓ | ✓ |
| `trash` | trash.py | 移入回收站（send2trash） | ✗ | ✓ | ✓ |
| `echo` | echo.py | 打印消息（模板） | ✓ | ✓ | ✓ |
| `write` | write.py | 写文本到文件（append/prepend/overwrite） | ✓ | ✓ | ✓ |
| `symlink` | symlink.py | 创建符号链接 | ✗ | ✓ | ✓ |
| `hardlink` | hardlink.py | 创建硬链接 | ✗ | ✓ | ✓ |
| `shell` | shell.py | 执行 shell 命令，返回 `{shell.output/returncode}` | ✓ | ✓ | ✓ |
| `extract` / `compress` | Shelfy 扩展 | ZIP 解压/压缩；解压支持密码列表；压缩支持 AES-256 密码；可删除原始压缩包/源文件 | ✗ | ✓ | compress ✓ |
| `python` | python.py | 执行 python 代码 | ✓ | ✓ | ✓ |
| `confirm` | confirm.py | 交互确认（StopIteration 中断管线） | ✓ | ✓ | ✓ |
| `macos_tags` | macos_tags.py | macOS Finder 标签 | ✗ | ✓ | ✓ |

冲突模式（`actions/common/conflict.py`）：`skip` / `overwrite` / `trash` / `rename_new` / `rename_existing` / `deduplicate`（内容相同则 skip，否则 rename）。默认 `rename_new`，模板 `{name} {counter}{extension}`，`next_free_name` 递增 counter 直到无冲突。

目标路径处理（`actions/common/target_path.py`）：`prepare_target_path`——以 `/` 结尾或 autodetect 无扩展名视为目录，创建父目录，若 dest 是已存在目录则拼 `dest/src_name`。

## 4. 配置 / CLI（`organize/config.py`, `cli.py`）

YAML 结构：
```yaml
rules:
  - name: "..."
    enabled: true
    targets: files  # files | dirs
    locations:
      - path: ~/Downloads
        min_depth: 0
        max_depth: inherit  # 或整数或 None；inherit 时随 rule.subfolders
        search: breadth
        exclude_files: [...]
        exclude_dirs: [...]
        filter: [...]        # 仅包含匹配的文件
        filter_dirs: [...]
    subfolders: false       # 等价于 max_depth=0
    tags: [...]
    filters:
      - extension: pdf
      - name: {contains: [Invoice], case_sensitive: false}
      - "not created": {days: 30}   # not 前缀反转
    filter_mode: all         # all | any | none
    actions:
      - move: {dest: "~/Documents/{extension}/{created.strftime('%Y-%m')}/", on_conflict: rename_new}
      - echo: "done"
```

默认排除（`location.py`）：文件 `thumbs.db desktop.ini ~$* .DS_Store .localized`；目录 `.git .svn`。

CLI（docopt）：`run` / `sim` / `new` / `edit` / `check` / `debug` / `show` / `list` / `docs`；选项 `--working-dir` / `--format default|jsonl|errorsonly` / `--tags` / `--skip-tags` / `--stdin`。

`should_execute`（tags 逻辑）：`always` 标签总是跑（除非被 skip）；`never` 标签不跑（除非被 tags 选中）；默认无 tags 时全跑。

## 5. Walker（`organize/walker.py`）

`scandir` 收集 dirs/nondirs（跳过 symlink），`os_sorted` 排序。`walk` 递归，`min_depth`/`max_depth` 控制，`filter_files`/`filter_dirs`（fnmatch 包含）、`exclude_files`/`exclude_dirs`（排除）。`files(path)` 单文件直接返回；`dirs(path)` 仅产目录。

## 6. 与 Shelfy 集成策略

Shelfy 的产品定位是本地优先的“文档/文件夹备份 + 规则化文件整理”工具。现有规则引擎（`rules.rs`）是简化版：DB 存规则（extensions + 单正则 pattern + destination + action），`process_file` 做匹配+move/ignore，watcher 实时触发，SQLite 记日志+undo。高级备份场景优先走 `orden` 的 `copy` 动作，以保留原件并写入备份目标。

集成方案：**`orden` 模块作为独立的高级引擎**，与现有 `rules.rs` 并存：
- 现有 DB 规则保留（简单模式，UI 管理），`orden` 模块支持加载 organize YAML 风格配置做高级自动化
- CLI 支持 `shelfy --cli orden|organize sim|run|check <config>` 子命令
- Tauri commands 支持 `orden_list/load/save/delete/check/run`
- Settings → Advanced 是 GUI 入口，可编辑 YAML、保存本地配置、校验、模拟、真实运行并查看日志
- `orden` 的 `move` / `rename` 动作执行成功后调用 `db::log_action_if_initialized` 写入 History，保持 Recent Actions / History / Undo 一致
- 其他动作（copy/delete/trash/shell/write/link）在定义可撤销语义前不写入现有可撤销 move 日志，避免 Undo 误处理

### 6.1 当前 Rust 内核实现快照

核心入口：

| 入口 | 文件 | 说明 |
|------|------|------|
| `Config::from_string` | `src-tauri/src/orden/mod.rs` | 解析 YAML 根对象和 `rules` 列表 |
| `Config::execute` | `src-tauri/src/orden/mod.rs` | 遍历规则，按 tags/skip-tags 判断是否执行 |
| `Rule::execute` | `src-tauri/src/orden/mod.rs` | location walk → filter pipeline → action pipeline |
| `run_yaml` | `src-tauri/src/orden/mod.rs` | GUI/command 友好入口，返回 summary + captured logs |
| `CollectingOutput` | `src-tauri/src/orden/action.rs` | 捕获日志给 Tauri 返回 |
| `DefaultOutput` | `src-tauri/src/orden/action.rs` | CLI/stderr 输出 |

配置存储：

- 存储目录：`<ProjectDirs data_dir>/orden/*.yaml`
- 配置名校验：只允许 ASCII 字母、数字、`-`、`_`；自动去掉 `.yaml` / `.yml` 后缀；拒绝 `/`、`\`、`..`
- `working_dir`：相对 location 会基于 `ExecuteOptions.working_dir` 解析；绝对路径保持原样
- 多来源：一条规则可配置多个 `locations`，单个 location 的 `path` 也可为列表，执行时依次遍历所有来源
- 多目的地：`copy.dest` 可为字符串列表，对每个匹配资源执行扇出复制；`move` 保持单目的地，避免首次移动后原文件不存在导致语义不确定

执行选项：

```rust
pub struct ExecuteOptions {
    pub simulate: bool,
    pub tags: HashSet<String>,
    pub skip_tags: HashSet<String>,
    pub working_dir: PathBuf,
}
```

Tauri 返回结构：

```rust
pub struct OrdenRunResult {
    success: u64,
    errors: u64,
    simulate: bool,
    logs: Vec<OrdenLog>,
}
```

### 6.2 当前已接入的前端/CLI 接口

Tauri commands：

```text
orden_list_cmd() -> Vec<String>
orden_load_cmd(name) -> String
orden_save_cmd(name, yaml)
orden_delete_cmd(name)
orden_check_cmd(yaml)
orden_run_cmd(yaml, simulate, tags, skip_tags) -> OrdenRunResult
```

CLI：

```bash
shelfy --cli orden check <config>
shelfy --cli orden sim <config> [--tags t1,t2] [--skip-tags t3] [--working-dir <dir>]
shelfy --cli orden run <config> [--tags t1,t2] [--skip-tags t3] [--working-dir <dir>]
```

兼容别名：`organize` 也会进入同一实现。

GUI：

- `src/store/useAppStore.ts` 提供 `ordenList/load/save/delete/check/run`
- `src/components/Settings.tsx` 的 `Advanced` tab 提供配置选择、配置名、Visual/Source 双模式 YAML 编辑器、tags/skip-tags、Check/Simulate/Run 和日志输出
- Visual 模式覆盖常见备份/整理规则；Source 模式保留完整 Orden YAML 能力

## 6.4 Scheduler / Cron / Keepalive

调度器继续复用 Shelfy 的 Clean Now 路径，但增加三个运行面：

- 固定时间：`schedule_enabled` + `schedule_time_1..4`，每天每个 slot 最多触发一次
- Cron：`schedule_cron_enabled` + `schedule_cron_expr`，支持 5 字段表达式 `minute hour day month weekday`
- Keepalive：`keepalive_enabled` + `keepalive_interval_minutes`，用于进程内 heartbeat 日志与前端事件

Cron parser 位于 `src-tauri/src/scheduler.rs`：

- 支持 `*`、`*/n`、`,` 列表、`a-b` 范围、`a-b/n` 步进
- minute `0-59`，hour `0-23`，day `1-31`，month `1-12`，weekday `0-7`
- weekday `7` 归一化为 Sunday `0`
- day-of-month 与 weekday 同时受限时使用传统 cron OR 语义
- `last_cron_minute` 防止同一分钟重复触发

本地日志：

- `scheduler_logs(id, timestamp, level, event, message, details)`
- 记录 `clean_started`、`clean_finished`、`clean_folder_failed`、`keepalive`、`keepalive_installed`、`keepalive_uninstalled`、`scheduler_error`
- Settings → General → Scheduler 可刷新和清空日志

OS 级保活：

- Windows：`schtasks /Create /SC MINUTE /MO <interval> /TN ShelfyKeepAlive /TR "<exe> --autostart" /F`
- macOS：写入 `~/Library/LaunchAgents/cc.shelfy.keepalive.plist`，包含 `RunAtLoad`、`StartInterval`、stdout/stderr 到 `~/Library/Logs/shelfy-keepalive*.log`，再用 `launchctl bootstrap`
- Linux：当前未安装 systemd user timer，仅支持进程内 keepalive；后续可补 `~/.config/systemd/user/shelfy-keepalive.service|timer`

## 6.5 MCP 接入策略

Shelfy 提供本地 stdio MCP 服务，入口：

```bash
shelfy --mcp
shelfy --cli mcp
```

Settings → General → MCP 负责：

- 显式开关 `mcp_enabled`
- 独立写权限 `mcp_allow_write`
- stdio command/args 快捷配置
- HTTP URL/token 字段，用于后续 HTTP bridge 或兼容本地 AI 运行时
- 生成常见 `mcpServers` 客户端配置片段

MCP tools：

- `shelfy_list_folders`
- `shelfy_list_rules`
- `shelfy_recent_logs`
- `shelfy_orden_simulate`
- `shelfy_scan_folder`（需要 `mcp_allow_write`）
- `shelfy_orden_run`（需要 `mcp_allow_write`）

安全边界：

- MCP disabled 时 `tools/list` 返回空列表
- 写工具单独受 `mcp_allow_write` 控制
- stdio 模式不监听网络端口；HTTP 字段只是客户端配置/桥接准备

### 6.3 History / Undo 约束

Shelfy 现有 History/Undo 以“从 destination rename 回 source”为核心模型，适合 `move` / `rename`。

当前策略：

- `move` / `rename` 真实执行时记录 `ActionLog { action, source_path, destination_path, file_name, file_type: "Orden" }`
- `simulate=true` 不写 History
- `copy`、`delete`、`trash`、`write`、`shell`、`symlink`、`hardlink` 暂不写 History，直到设计对应的“可撤销/不可撤销事件”模型

后续可选方案：

- 扩展 `action_logs` 增加 `reversible` / `metadata` 字段
- 对不可撤销动作只展示审计记录，不提供 Undo
- 对 `copy` 的 Undo 删除复制件；对 `trash` 尝试系统回收站恢复；对 `delete/shell/write` 默认不可撤销

## 7. Rust 实现复用现有 crate（避免重复造轮子）

| 功能 | organize-tool (Py) | Rust crate |
|------|--------------------|------------|
| glob 匹配 | fnmatch / simplematch | `globset` |
| 正则 | re | `regex` |
| 哈希 | hashlib | `sha1` `sha2` `md-5` + `hex` |
| MIME | mimetypes | `mime_guess` |
| 回收站 | send2trash | `trash` |
| YAML | pyyaml | `serde_yaml` |
| 日期 | arrow / datetime | `chrono` |
| 文本提取 | pdftotext/pdfminer/docx2txt | 文本/代码/配置直接读 + 探测；PDF 调 `pdftotext`；DOCX 读 zip XML |
| ZIP 压缩/解压 | zipfile/第三方工具 | `zip`（读写、AES/ZipCrypto 解密、AES-256 加密） |

**自实现（无合适现成 crate / 需兼容 organize 语法）**：
- **模板引擎**：jinja2 的 `{var.method()}` + `{dt.strftime()}` 语法，`tera`/`minijinja` 用 `|filter` 语法无法兼容 organize YAML 配置 → 自写轻量渲染器（已实现 `template.rs`）
- **filecontent 提取**：已扩展为常见文本/代码/配置扩展名直接读，Dockerfile/Makefile/.gitignore 等扩展名less 文本按文件名识别，未知小文件用二进制控制字符启发式探测；单文件默认上限 16MB；pdf 优先调系统 `pdftotext`（poppler）；docx 读 zip 内 word/document.xml 提取文本（纯 Rust 可做）
- **archive 动作**：作为 Shelfy 扩展实现 `extract`/`compress`。当前聚焦 ZIP：解压支持密码列表自动尝试，解压/压缩都会把操作日志写入 Orden 执行结果；压缩支持 AES-256 密码；`delete_original` 可删除原始压缩包或源文件。RAR/7z 后续可通过 7z/unrar 外部命令或新增 crate 扩展。
- **duplicate 检测算法**：复刻 size→chunk hash→full hash 三级筛选逻辑

## 8. 风险与决策

- **python/shell 过滤器**：Rust 无法 exec python 代码；`python` 过滤器暂不支持（或仅在系统有 python 时 `std::process::Command` 调用），`shell` 动作可直接 `std::process::Command::sh`
- **exif**：纯 Rust EXIF 读取用 `kamadak-exif` crate（JPEG/TIFF）；复杂格式回退 `exiftool` 外部命令。已在 `Cargo.toml` 待加。
- **macOS 专属**：`date_added`/`date_lastused`/`macos_tags` 用 `mdls` / Finder API；跨平台时返回错误或跳过
- **datetime 创建时间**：Unix `st_birthtime` 需用 `std::fs::Metadata`（Rust 稳定版不直接暴露 birthtime），需用 `nix` 或 `libc` 调 `stat`；或回退 mtime。决策：优先 `std::fs::metadata().created()`，失败回退 mtime 并 warn
