# Shelfy UI_SPEC

## Product Surface

Shelfy is a quiet cross-platform backup and file organizer. It supports document/folder backup flows and rule-based file organization from the same operational surface. The first screen is the working tray popup, not a marketing page. Settings are dense, scannable, and optimized for repeated use.

## Design System

- Component baseline: coss/shadcn-style React components in `src/components/ui/*`.
- Native-looking controls are not allowed in product screens. Buttons, selects, switches, checkboxes, sliders, labels, badges, cards, and separators must come from the local shadcn component layer.
- Styling uses CSS variables in `src/index.css` and Tailwind utility classes. No component may hard-code a one-off visual system.
- Cards are limited to repeated items, grouped controls, and toasts. Page sections use plain layout bands and spacing.

## Visual Tokens

- Radius: `6px` base, `8px` for cards and major controls, with full pills reserved for switches, progress tracks, and status dots.
- Primary: pine green `#1F4A37`, used for primary actions, active navigation, focus, and brand mark.
- Accent: sage `#6F8A69` and emerald `#1E7A5A`, reserved for secondary emphasis and informational states.
- Warm surfaces: cream `#F2E6C9`; gold `#E9CF8A` / `#C9A23D` is limited to restrained highlights.
- Destructive: rose/red, used only for delete, clear, and irreversible actions.
- Neutral surfaces: background, card, popover, muted, border, and input tokens.
- Typography: system UI stack, 13-16px for operational surfaces, no viewport-scaled type.

## Iconography

- Product icon: the rounded-rectangle SHELFY pinecone badge in `src-tauri/icons/app-icon.png`; its smaller 80% body preserves platform-safe spacing.
- Tray icon is the only squirrel mark and remains a monochrome template asset in `src-tauri/icons/tray-icon.*`.
- All UI icons must be imported from `lucide-react`.
- Animated icon behavior must use the `AnimatedIcon` wrapper, which follows the lucide-animated direction: subtle Motion-based hover/tap movement, reduced-motion aware.
- Icons inside buttons are required when a Lucide symbol exists for the command.

## Layout

- Popup width is compact and action-first: brand/header, clean controls, pending state, recent actions, weekly stats, toasts.
- Settings uses a fixed sidebar plus scrollable content region.
- Tables/lists should prefer compact rows with stable icon/button dimensions.
- Text must truncate in path-heavy rows instead of forcing layout expansion.

## Interaction

- Focus states use `ring` tokens from shadcn variables.
- Scrollable product surfaces use the token-based thin scrollbar from `src/index.css`: transparent track, rounded muted thumb, primary hover, and ring active state. Compact horizontal navigation and step rails may hide the thumb while preserving wheel, trackpad, and keyboard scrolling.
- Icon-only controls use the local shadcn/Radix tooltip component, never browser-native `title` tooltips.
- Folder and destination paths can be typed or selected with the platform dialog.
- Rule editing supports priority, extension matching, optional regex pattern, destination, action, target folder scope, and enabled state.
- Scheduler editing supports fixed daily times, 5-field cron expressions, Windows/macOS system keepalive install/remove, and scheduler log review.
- Long-running actions must expose disabled/loading state and refresh store data after completion.
- Custom-decorated Settings and tray popup windows expose a visible top-center drag handle; the surrounding header whitespace remains draggable while controls remain clickable.
- Orden run-history rows open a desktop dialog. Large structured log sets are searched client-side and rendered in pages instead of expanding hundreds of rows inline.
- The Orden configuration center searches names and notes and renders at most six configurations per page on both table and card layouts.

## Data Model

- SQLite is the source of truth for settings, watched folders, rules, logs, and scheduler state.
- Scheduler settings include fixed-time scheduling, cron scheduling, in-process keepalive intervals, and OS-level keepalive install commands for Windows Task Scheduler and macOS LaunchAgent.
- Scheduler logs are stored in `scheduler_logs` and surfaced in Settings for clean runs, cron/fixed-time triggers, keepalive heartbeats, install/remove events, and failures.
- JSON configuration snapshots are supported for external management and migration.
- Rule JSON import/export remains available for sharing just the rule set.
- Advanced YAML rules are stored as files under the app data directory in `orden/*.yaml`.
- Orden run results return a summary plus structured logs; `move` and `rename` actions write to the existing History model, while other advanced actions remain non-history until explicit undo semantics are defined.

## CLI Contract

- The packaged binary supports a local CLI for external callers:
  - `shelfy --cli scan <folder>`
  - `shelfy --cli rules list|export <path>|import <path> [--replace]`
  - `shelfy --cli folders list|add <path> [mode]|remove <id>|mode <id> <mode>`
  - `shelfy --cli config path|export <path>|import <path> [--replace]`
  - `shelfy --cli orden check <config>`
  - `shelfy --cli orden sim|run <config> [--tags t1,t2] [--skip-tags t3] [--working-dir <dir>]`
- CLI output is JSON where practical, with human-readable errors on stderr.
- `organize` is accepted as a compatibility alias for the `orden` CLI namespace.

## Advanced Rules

- Settings includes an Advanced tab for the built-in Orden YAML engine.
- User-facing Orden terminology is outcome-oriented: configuration → organization plan, rule → sorting rule, filter → match condition, action → file operation, and job/task → automatic run. Chinese uses “整理方案 / 分类规则 / 匹配条件 / 处理方式 / 自动运行”. Technical storage and API field names remain stable.
- Internal enum values such as `manual`, `fixed`, `mcp-run`, action names, senders, and levels must be rendered through i18n label helpers. Raw identifiers are allowed in YAML/Source mode and diagnostics, but not as normal Visual-mode labels.
- The Advanced tab must expose saved config selection, config name, YAML editor, Visual/Source mode switching, tags, skip-tags, Check, Simulate, Run, and structured log output.
- Visual mode is for common backup/organizing rules and must serialize back to source YAML; Source mode remains the escape hatch for advanced Orden syntax.
- Visual mode location fields support system picker buttons for multiple files or multiple folders. Destination fields support system picker buttons for multiple destination folders.
- The Orden engine is the preferred path for document/folder backup workflows because `copy` can preserve originals while writing into a backup destination.
- Long-running Orden actions must disable action buttons while running and refresh History/Stats after a real run.
- Config names must be treated as names, not paths; path separators and `..` are invalid.
- Basic Rule creation and editing uses a focused desktop dialog over the rule list. Keep the editor grouped and compact; do not turn it into a route-like web page.

## MCP

- Settings includes an MCP section for local AI integration.
- MCP has an explicit enable switch. When disabled, the stdio server exposes no tools.
- The preferred local transport is stdio with command args `--mcp`; HTTP configuration is stored for clients that proxy or expose Shelfy through an HTTP MCP bridge.
- The generated client config follows the common `mcpServers` shape with server name, command/args or URL/token.
- Write-capable tools are guarded by a separate `mcp_allow_write` switch.
- The stdio MCP server currently exposes folder/rule/log inspection, Orden simulation, and optional write tools for folder scan and Orden run.

## Scheduler

- Fixed-time mode supports 1-4 local times per day and runs the same silent-folder cleaning path as Clean Now.
- Cron mode accepts a standard 5-field expression: `minute hour day month weekday`.
- Cron fields support `*`, `*/n`, comma lists, numeric ranges, and stepped ranges. Weekday `0` and `7` both mean Sunday.
- If both day-of-month and weekday are constrained, Shelfy uses normal cron OR semantics.
- In-process keepalive writes heartbeat records while Shelfy is running.
- OS-level keepalive is installed separately: Windows uses `schtasks` with `ShelfyKeepAlive`; macOS writes `~/Library/LaunchAgents/cc.shelfy.keepalive.plist` and bootstraps it with `launchctl`.
- Linux currently keeps the in-process heartbeat and settings/log UI, but no OS-level keepalive installer is exposed.
