# Release Process

## Version Bump

版本号记录在 **三个文件** 中，必须同步更新：

| 文件 | 字段 |
|---|---|
| `package.json` | `version` |
| `src-tauri/tauri.conf.json` | `version` |
| `src-tauri/Cargo.toml` | `version` |

## 发布步骤

```bash
# 1. 修改以上三个文件的版本号（如 0.1.5 → 0.1.6）

# 2. 提交
git add -A
git commit -m "chore: bump version to 0.1.6"

# 3. 打 tag（tag 名必须与版本号一致，前缀 v）
git tag v0.1.6

# 4. 推送（先推 commit，再推 tag）
git push
git push origin v0.1.6
```

## CI 流程

推送 tag 后自动触发 `.github/workflows/build.yml`：

1. **validate** — 检查三个文件的版本号是否都与 tag 一致（如 `v0.1.6` → 三个文件均应为 `0.1.6`）。**不匹配则直接失败**
2. **publish** — 在 Windows / Linux / macOS 三平台分别构建，生成 `.dmg` / `.msi` / `.AppImage` 等安装包
3. 构建产物自动上传到 GitHub Release（草稿 → 正式发布）

## 注意事项

- tag 名必须严格 `v` + 语义化版本号，如 `v0.1.5`
- 推送前务必确认三个文件的版本号一致
- 如果 push 后发现 validate 失败，修复后需要**先删除远端 tag** 再重新打 tag：
  ```bash
  git tag -d v0.1.5
  git push origin :v0.1.5
  # 修复版本号后提交新 commit
  git tag v0.1.5
  git push origin v0.1.5
  ```
- 历史 release 产物不可覆盖（GitHub Release 有版本记录）
