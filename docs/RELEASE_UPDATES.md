# Shelfy 发布与应用内更新

## 发布契约

版本号必须在 `package.json`、`src-tauri/Cargo.toml` 与 `src-tauri/tauri.conf.json` 保持一致，tag 使用 `vX.Y.Z`。推送 tag 后，`.github/workflows/build.yml` 构建 Windows、Linux 和 macOS Universal 资产并创建 GitHub Release。

macOS Release 必须包含：

- `Shelfy_vX.Y.Z_universal-apple-darwin.app.zip`：版本化归档，用于追踪和人工核查。
- `Shelfy_universal-apple-darwin.app.zip`：稳定归档；应用内更新与 Homebrew Cask 必须共同使用这个端点。
- `latest.json`：固定入口为 `https://github.com/mcxen/shelfy/releases/latest/download/latest.json`。
- 原有 Universal DMG 与 `.app.tar.gz` 手动安装资产。

`latest.json` 的格式为：

```json
{
  "version": "0.3.0",
  "tag": "v0.3.0",
  "platform": "macos",
  "target": "universal-apple-darwin",
  "asset_name": "Shelfy_universal-apple-darwin.app.zip",
  "asset_url": "https://github.com/mcxen/shelfy/releases/latest/download/Shelfy_universal-apple-darwin.app.zip",
  "sha256": "<64-character lowercase hex>",
  "size": 12345678,
  "notes": ""
}
```

## 用户流程

Settings → General → 软件更新支持：

1. 手动检查当前版本和最新版本。
2. 只有清单、平台、URL、SHA256、大小和本地安装位置都通过校验时，才显示“下载、安装并重启”。
3. 用户可开启“自动安装更新”；默认关闭。Release 构建启动后延迟检查，确认可安装时自动完成同一流程。
4. Windows/Linux 当前只检查版本并提示手动安装；自动替换仅支持 macOS `.app`。

下载和解压在 Shelfy cache 的 `updates/` 下完成。主进程只接受固定的 stable URL，并验证下载大小与 SHA256；helper 是当前主程序的临时副本，会再次校验 staging 路径、bundle id、版本和主可执行文件。它等待主进程退出，将原 `Shelfy.app` 同卷移动为隐藏备份，再复制新 bundle。新 bundle 无法准备或启动时恢复备份，并尝试重新打开旧版本。

如果当前用户不能替换 `Shelfy.app`，应用内自动安装会停止，不会请求提权。Homebrew 管理的安装应使用：

```bash
brew tap mcxen/shelfy https://github.com/mcxen/shelfy.git
brew install --cask mcxen/shelfy/shelfy
brew upgrade --cask --greedy-latest mcxen/shelfy/shelfy
```

仓库内的 `Casks/shelfy.rb` 使用同一个 stable URL。由于 Cask 使用 `version :latest`，升级命令必须带 `--greedy-latest`。归档是 Universal 2；Tauri bundle 当前声明 Intel 最低 macOS 10.13、Apple Silicon 最低 macOS 11。

当前发布没有 Developer ID 身份，只在打包和更新替换阶段进行免费 ad-hoc 重签，并用 `codesign --verify --deep --strict` 检查 bundle 完整性；未声明 Apple notarization。helper 和 staging bundle 也会清理 quarantine 扩展属性并重新 ad-hoc 签名，这不等同于开发者身份签名或公证。Homebrew 首次安装若被 Gatekeeper 阻止，Cask caveat 会引导用户在“系统设置 → 隐私与安全性”审阅警告后使用“仍要打开”。

## 发布前检查

- `npm run build`
- `npm run verify:release-contract`
- `cargo test updater::tests -- --nocapture`（workdir `src-tauri`）
- `cargo build`（workdir `src-tauri`）
- Universal bundle 构建后运行 `npm run verify:macos-release`。
- `git diff --check`
- 确认 tag 与三处版本一致。
- 确认工作流上传版本化 ZIP、稳定 ZIP 和 `latest.json`，并确认 Cask、manifest 与 updater 的 stable URL 完全一致。
- 发布后从真实 `.app` 验证检查、下载、替换、重启与失败回滚；不要在开发构建中替换真实安装。

`brew style Casks/shelfy.rb` 当前会对 `releases/latest/download/*.zip` 报一条 `Cask/Url`，因为该规则只豁免带显式 tag 的 `/releases/download/` ZIP。这里有意保留 stable URL，以满足 Cask 与应用内 updater 共用同一端点的发布契约；其余 stanza 顺序和 Ruby 语法均通过检查。
