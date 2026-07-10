# TASK - Orden 内核技术方案与任务拆解

> 在 Shelfy (`src-tauri/src/orden/`) 用 Rust 复刻 organize-tool v3 的核心能力，集成进现有后端，之后再独立成 crate。
> 调研见 `RESEARCH.md`。

## 设计概要

```
src-tauri/src/orden/
├── mod.rs              # 模块入口 + Config/Rule 解析与执行（YAML→serde）
├── value.rs            # 动态 Value 类型（Str/Int/Float/DateTime/Map，模板变量载体）✓
├── resource.rs         # Resource（被处理的文件 + vars 共享态）✓
├── template.rs         # organize 语法模板引擎 {var.key.method()} ✓
├── walker.rs           # 目录遍历（globset，深度控制，排除）✓
├── filter.rs           # Filter trait + All/Any/Not 组合器 + run_pipeline ✓
├── action.rs           # Action trait + Output trait ✓
├── location.rs         # Location 配置 + 默认排除规则 ✓
├── conflict.rs         # 冲突解决 skip/overwrite/trash/rename_new/rename_existing/deduplicate ✓
├── target_path.rs      # 目标路径处理（目录探测/创建）✓
├── filters/
│   ├── mod.rs          # 过滤器注册表 + 工厂 from_yaml ✓
│   ├── extension.rs    # ✓
│   ├── name.rs         # ✓
│   ├── regex.rs        # ✓
│   ├── size.rs          # ✓
│   ├── empty.rs         # ✓
│   ├── mimetype.rs      # ✓
│   ├── hash.rs          # ✓
│   ├── duplicate.rs     # ✓
│   ├── created.rs       # ✓
│   ├── lastmodified.rs  # ✓
│   ├── filecontent.rs   # 文本/代码/配置广泛识别；pdf 调 pdftotext；docx 读 zip xml ✓
│   └── exif.rs          # kamadak-exif + exiftool 回退 ✓
└── actions/
    ├── mod.rs          # 动作注册表 + 工厂 from_yaml ✓
    ├── move_.rs        # move；真实执行后写 History ✓
    ├── copy.rs         # ✓
    ├── rename.rs       # rename；真实执行后写 History ✓
    ├── delete.rs       # ✓
    ├── trash.rs        # ✓
    ├── echo.rs         # ✓
    ├── write.rs        # ✓
    ├── archive.rs      # extract/compress zip；密码本；删除原包/源文件 ✓
    ├── symlink.rs      # ✓
    ├── hardlink.rs     # ✓
    └── shell.rs        # ✓
```

## 当前内核接口

### Rust API

- `orden::Config::from_string(yaml)`：解析 YAML 配置。
- `orden::Config::execute(&ExecuteOptions, &dyn Output)`：执行配置并返回 `ReportSummary`。
- `orden::run_yaml(yaml, &ExecuteOptions)`：Tauri/调用方常用入口，返回 `RunResult { success, errors, simulate, logs }`。
- `orden::list_config_names/load_config_text/save_config_text/delete_config`：管理 `<data_dir>/orden/*.yaml`。配置名只允许字母、数字、`-`、`_`，禁止路径分隔符和 `..`。
- `ExecuteOptions { simulate, tags, skip_tags, working_dir }`：控制 dry-run、tags 过滤和相对 location 的解析基准。

### Tauri Commands

- `orden_list_cmd() -> Vec<String>`
- `orden_load_cmd(name) -> String`
- `orden_save_cmd(name, yaml)`
- `orden_delete_cmd(name)`
- `orden_check_cmd(yaml)`
- `orden_run_cmd(yaml, simulate, tags, skip_tags) -> OrdenRunResult`

### CLI

```bash
shelfy --cli orden check <config>
shelfy --cli orden sim <config> [--tags t1,t2] [--skip-tags t3] [--working-dir <dir>]
shelfy --cli orden run <config> [--tags t1,t2] [--skip-tags t3] [--working-dir <dir>]
```

兼容别名：`shelfy --cli organize ...` 会进入同一套 `orden` 实现。

### GUI

Settings → Advanced 是当前图形入口，支持：

- 读取/保存/删除本地 YAML 配置。
- 直接编辑 YAML。
- Check / Simulate / Run。
- 传入 tags / skip-tags。
- 展示 `CollectingOutput` 捕获的执行日志。

## 任务进度

### 阶段 0：依赖与骨架
- [x] 更新 `Cargo.toml`（serde_yaml/globset/sha1/sha2/md-5/hex/mime_guess/trash/anyhow/zip/kamadak-exif）— encoding 探测后续按需补
- [x] 创建模块目录
- [x] `value.rs` 动态 Value 类型
- [x] `resource.rs` Resource
- [x] `template.rs` 模板引擎（含单元测试）
- [x] `walker.rs` 遍历器
- [x] `filter.rs` Filter trait + 组合器
- [x] `action.rs` Action trait + Output

### 阶段 1：过滤器
- [x] extension
- [x] name（含 glob 测试）
- [x] regex（命名组）
- [x] size
- [x] empty
- [x] mimetype
- [x] hash（sha1/sha2/md5）
- [x] duplicate（size→chunk→full 三级）
- [x] TimeFilter 公共逻辑（years..seconds + older/newer）
- [x] created（std created()，失败回退 mtime）
- [x] lastmodified（mtime）
- [x] filecontent（常见文本/代码/配置 + 扩展名less 文本探测 + pdf pdftotext + docx zip xml）
- [x] exif（kamadak-exif + exiftool 回退）

### 阶段 2：动作
- [x] conflict.rs 冲突解决（6 模式 + next_free_name）
- [x] target_path.rs 目标路径处理
- [x] echo
- [x] move
- [x] copy
- [x] rename
- [x] delete
- [x] trash（trash crate）
- [x] write（append/prepend/overwrite）
- [x] symlink
- [x] hardlink
- [x] shell（std::process::Command sh -c）
- [x] extract / compress（zip 解压/压缩、密码列表、压缩密码、删除原始文件）

### 阶段 3：配置与执行
- [x] location.rs Location + 默认排除
- [x] filters/mod.rs 注册表 + from_yaml 工厂（支持 `not ` 前缀、str/dict/value 三种形式）
- [x] actions/mod.rs 注册表 + from_yaml 工厂
- [x] mod.rs Config（YAML 反序列化）+ Rule.execute（walk→filter→action pipeline）
- [x] 单条规则支持多个来源目录（`locations` 与 `location.path` 列表）
- [x] copy 支持 `dest` 列表，一次匹配扇出复制到多个目的地
- [x] 过滤条件组合支持 `filter_mode: all|any|none`（AND / OR / NONE）
- [x] tags/skip-tags 执行逻辑（should_execute）
- [x] simulate 模式

### 阶段 4：集成
- [x] `lib.rs` 注册 `pub mod orden;`
- [x] CLI 增加 `orden|organize sim|run|check <config>` 子命令
- [x] Tauri commands 增加 `orden_list/load/save/delete/check/run`
- [x] Settings 增加 Advanced/Orden YAML 配置面板（保存、校验、模拟、运行、日志）
- [ ] 动作执行后写 `db::log_action` 保持 UI 历史/undo 一致（move/rename 已接入；copy/delete/trash/shell 需设计 undo 语义）
- [x] 与现有 `rules.rs` 并存（简单 DB 规则继续用，YAML 高级规则走 orden）
- [x] 基础 Rules 重构为多规则配置表，支持创建、编辑、更新、保存、启停、历史和删除，并自动生成功能说明

### 阶段 5：验证
- [x] `cargo test` 编译和单元测试通过（当前 17 个测试）
- [x] 前端 `npm run build` 通过
- [x] 端到端：临时示例 YAML `orden sim` 通过（`success=1, errors=0`）
- [x] 多来源移动脚本 `scripts/test-orden-multi-source-move.sh`：创建 3 个来源及 6 个文件，全部汇总移动到单一新目录
- [ ] 端到端：示例 YAML `orden run` + History/Undo
- [ ] 与 Shelfy watcher 协同测试

### 阶段 6：调度 / Cron / 后台保活
- [x] SQLite settings 增加 cron 与 keepalive 配置项
- [x] SQLite 增加 `scheduler_logs`，记录调度、清理、保活、安装/卸载与错误事件
- [x] Rust scheduler 支持固定时间 + 5 字段 cron 并行触发
- [x] Settings UI 支持 cron 校验、后台保活间隔、日志刷新/清空
- [x] Windows Task Scheduler keepalive 安装/卸载接口
- [x] macOS LaunchAgent keepalive 安装/卸载接口
- [ ] Linux systemd user timer 支持（当前仅保留进程内 keepalive 与日志）

### 阶段 7：交互式 YAML / MCP
- [x] Advanced/Orden 增加 Visual / Source 双模式编辑
- [x] Visual 模式支持常见备份/整理规则字段并序列化回 YAML
- [x] Visual 模式支持系统选择器多选文件、多选目录、多选目的地
- [x] Visual 模式支持 AND / OR / NONE 过滤关系，并生成多来源、多目的地 YAML
- [x] 后端提供 YAML → Visual model 解析接口
- [x] Advanced/Orden 重构为配置中心：配置与自动化任务分别按表格逐行管理，编辑器/详情/运行结果使用独立视图
- [x] Orden 自动化任务支持 manual/fixed/cron/interval/monitor 的创建、编辑、启停、立即运行与删除
- [x] 单条 Orden 配置支持行内试跑、配置预览和分组更多菜单；可视化规则按基础信息、来源筛选、执行动作 Card 归组
- [x] Settings 增加 MCP 开关、stdio/HTTP 快捷配置和客户端配置片段
- [x] 新增 `shelfy --mcp` / `shelfy --cli mcp` stdio MCP 服务
- [x] MCP 写工具由独立 `mcp_allow_write` 控制
- [ ] Visual 模式保留复杂 YAML 的 round-trip 注释/未知字段
- [ ] HTTP MCP server/bridge 实现

### 阶段 8：桌面界面与运行性能
- [x] Settings 从左侧栏改为顶部浮动导航，窗口外壳与 Popup 使用克制的玻璃质感
- [x] 压缩主内容边距和无效留白，General/Ignore/Orden Preview 使用统一 Card 与响应式布局
- [x] Popup / Settings 使用动态分包，Settings 数据改为进入对应 tab 后再加载
- [x] Orden 配置中心仅预取每份配置最近一次结果，进入详情后再加载完整历史与 YAML
- [x] Popup 后台轮询从 5 秒全量刷新拆为 15 秒 pending / 60 秒 Orden tasks，并在窗口失焦时停止
- [x] 移除未使用的隐藏 `main` WebView，自启动保持零窗口直到用户打开 UI
- [x] tray 菜单增加监控状态、主面板、立即整理、Orden 自动化、监控目录和设置；语言/文件夹变化时刷新
- [x] 除 Tray 小松鼠外，桌面端、移动端、安装包、网页 favicon 与界面品牌标记统一替换为松果圆角矩形图标
- [x] 全局 UI 主题切换为深松针绿/鼠尾草绿/暖奶油品牌色，浅深模式同步，并将基础圆角收敛到 6px

## 关键技术决策

1. **模板引擎自写**：兼容 organize `{var.method()}` 语法，不引入 tera/minijinja（见 RESEARCH.md §7）
2. **Filter/Action 用 trait object**：`Box<dyn Filter>` / `Box<dyn Action>`，工厂从 YAML 构造
3. **YAML 反序列化**：自定义 `deserialize`，因 organize 支持多种简写（`- extension` / `- extension: pdf` / `- extension: [pdf,docx]`）
4. **python 过滤器**：暂不实现（Rust 无法 exec python 代码）
5. **filecontent**：pdf 优先系统 `pdftotext`，docx 用 `zip` + xml 解析（纯 Rust）
6. **datetime 创建时间**：`std::fs::metadata().created()`，失败回退 mtime + warn

## 当前阻塞

- 暂无编译阻塞。`cargo test` 与前端 `npm run build` 已通过。
- 品牌图标与主题替换已通过 `npm run build`、`cargo build`；Tray 资产哈希校验保持不变。
- 剩余设计点：非 move/rename 的 orden 动作如何进入 Shelfy History/Undo 需要单独定义语义，避免 copy/delete/trash/shell 被现有“反向 rename”撤销逻辑误处理。

## 下一步

1. 给 Advanced/Orden 配置中心和任务表补齐其它语言文案（当前中英文 key 完整，其它语言走 fallback）
2. 为 copy/delete/trash/shell 设计 History/Undo 策略，或显式标记为不可撤销事件
3. 将 watcher/scheduler 的高级模式接入 orden 配置（当前 Settings/CLI/手动运行已接入）
4. 增加端到端测试：保存配置 → sim → run → History/Undo
5. 设计专用 Backup UI：在 Advanced/Orden 的 `copy` 能力之上提供文档/文件夹备份模板
6. 为 MCP 增加客户端兼容性测试（Claude Desktop、LocalAI、其它本地运行时）
