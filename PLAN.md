# cogit 100轮优化计划 — gix + JetBrains Git 管理

## 总目标

将 cogit 从 git2(libgit2) 全面迁移到 gix(读) + git CLI shell-out(写)，实现完整的 JetBrains IDE Git 集成体验。

---

## Phase 1: 基础设施迁移 (git2 → gix + shell git)

### 1.1 Cargo.toml 依赖替换
- 移除 `git2 = "0.20"` 和 `tokio`
- 添加 `gix = { version = "0.70", features = ["max-performance", "blocking-network-client"] }`
- 保留: ratatui, crossterm, serde, anyhow, thiserror, clap, regex, chrono, unicode-*, config, directories

### 1.2 gitops/repo.rs — Repo 结构体重写
- `inner: git2::Repository` → 使用 `gix::Repository` (gix::discover 或 gix::open)
- `path()` 保留
- `head_shorthand()` 用 gix 的 head 获取

### 1.3 gitops/shell.rs — 统一 shell-out 核心 (新增)
```rust
// 所有写操作通过 git CLI shell-out
fn git_cmd(repo_path: &Path) -> Command { ... }
fn run_git(cmd: &mut Command) -> Result<String, GitError> { ... }
```

### 1.4 gitops/status.rs — 用 gix 重写
- 用 gix 的 status/diff 功能读文件状态
- 保留 `FileStatus` 和 `WorktreeFile` 类型不变

### 1.5 gitops/index_ops.rs — 改为 shell-out
- `stage_path` → `git add <path>`
- `unstage_path` → `git reset HEAD -- <path>`
- `stage_all` → `git add -A`
- `unstage_all` → `git reset HEAD`
- `discard_path` → `git checkout -- <path>`

### 1.6 gitops/commit.rs — 改为 shell-out
- `commit` → `git commit -m <msg>`
- `commit_amend` → `git commit --amend -m <msg>`
- `cherry_pick` → `git cherry-pick <oid>`
- `cherry_pick_abort` → `git cherry-pick --abort`
- `cherry_pick_continue` → `git cherry-pick --continue`

### 1.7 gitops/branch.rs — 改为 shell-out + gix 读
- 读: gix 获取分支列表、upstream 信息
- 写: `git checkout`, `git branch`, `git branch -d/-D`, `git branch --set-upstream-to`

### 1.8 gitops/remote.rs — 改为 shell-out
- `remotes()` → `git remote -v` 解析
- `fetch` → `git fetch <remote>`
- `push` → `git push <remote> <refspec>`

### 1.9 gitops/merge_rebase.rs — 改为 shell-out
- `merge` → `git merge <branch>`
- `rebase` → `git rebase <branch>`
- `rebase_continue` → `git rebase --continue`
- `rebase_abort` → `git rebase --abort`
- `rebase_skip` → `git rebase --skip`
- `pull` → `git pull <remote> <branch>` (支持 --rebase)
- 状态检测改为检查 `.git/rebase-merge/` 或 `.git/rebase-apply/` 目录

### 1.10 gitops/stash.rs — 改为 shell-out
- `stash_save` → `git stash push -m <msg>` / `git stash push -u -m <msg>`
- `stash_pop` → `git stash pop stash@{N}`
- `stash_apply` → `git stash apply stash@{N}`
- `stash_drop` → `git stash drop stash@{N}`
- `stash_list` → `git stash list` 解析

### 1.11 gitops/shelve.rs — shell-out + 文件操作
- 使用 `git diff > .cogit/shelves/<name>.patch` 生成 patch
- 使用 `git apply <patch>` 恢复
- 保留 `.cogit/shelves/` 目录结构

### 1.12 gitops/mod.rs — 更新错误类型
- `Git2` → `Gix` (from gix::Error)
- 新增 `Shell(String)` 错误变体 (git CLI stderr)
- 保留其他变体

---

## Phase 2: Vim 键位完善

### 2.1 vimkeys.rs — 完整 Vim Motion 支持
- **hjkl** 基础移动
- **gg/G** 跳顶/跳底 (需要 pending key 状态机)
- **Ctrl+u/Ctrl+d** 半页翻滚
- **Ctrl+b/Ctrl+f** 整页翻滚
- **w/b/e** 词移动 (在列表中映射为下一个/上一个 section)
- **0/$** 行首/行尾
- **数字前缀** `3j`, `5k`, `2Ctrl+d` 等 (需要 count 累积状态)
- **/ 搜索** + **n/N** 下/上一个匹配
- **%** 跳到匹配括号 (在 diff 中跳到对应 hunk)
- **zz** 当前行居中
- **zt/zb** 当前行置顶/置底
- **ma-mz** 标记 (mark) + `'a` 跳到标记
- **dd** 删除当前项 (discard file / delete branch)
- **yy** 复制路径
- **p** 粘贴

### 2.2 Pending Key 状态机
```rust
pub struct VimStateMachine {
    pending: Vec<char>,     // 累积按键
    count: Option<usize>,   // 数字前缀
}
```
支持:
- `gg` (先按 g, 等待第二个 g)
- `dd` (先按 d, 等待第二个 d)
- `3j` (先按 3, 等待 j)
- `zt`, `zz`, `zb`
- `]c`, `[c` (diff hunk 导航)

### 2.3 Mode 支持
- **Normal**: 默认模式，所有 vim motion
- **Visual**: `v` 进入，支持 visual select + 批量操作
- **Command**: `:` 进入，命令行输入 (`:w`, `:q`, `:branch checkout main`, `:log`, `:stash`, `:rebase main`)
- **Insert**: 仅在 commit message 编辑时使用

---

## Phase 3: JetBrains Git 管理模式

### 3.1 智能分支切换 (Smart Checkout)
- 切换前自动检测未提交更改
- 如有更改，自动 stash → checkout → stash pop
- 如 pop 有冲突，提示用户选择: 保持双方 / 取消
- UI: sidebar 选中分支后按 `c` 弹出确认对话框

### 3.2 搁置管理 (Shelve)
- JetBrains 风格: 将更改存为命名 patch 文件
- `Shelve Changes...` 对话框: 输入名称 + 选择文件
- Shelves 列表在 sidebar 展示
- Unshelve: 应用 patch (不删除) 或 Restore (应用并删除)
- Unshelve 支持与当前更改合并

### 3.3 .gitignore 管理
- 选中文件按 `i` → 添加到 .gitignore
- 弹出对话框确认 pattern
- 支持追加到最后一个未注释的 .gitignore 文件
- 支持 `.gitignore` 列表查看/编辑面板

### 3.4 Rebase/Merge 从远端拉取
- `git pull --rebase` 作为默认 pull 策略
- 交互式 rebase 支持 (编辑、squash、reword、drop)
- Merge 冲突检测 + 三方对比视图
- Pull 前自动 fetch + 预览 incoming commits

### 3.5 Commit 管理
- Commit and Push: `C` (大写) 一键提交并推送
- Amend Commit: 修改上次提交信息
- Commit 模板/历史: 保存最近 N 条 commit message

### 3.6 Log 查看 (新增面板)
- `git log --oneline --graph` 可视化
- 选中 commit 显示详细 diff
- 支持搜索 commit message
- 支持按 author/date 过滤

---

## Phase 4: 新面板 + UI 增强

### 4.1 Branch Panel (新面板)
- Tab 切换: Files → Branches → Log → Stash
- 本地/远端分支列表
- 分支操作: 新建、删除、重命名、设置 upstream
- 分支间 diff 预览

### 4.2 Log Panel (新面板)
- git log 图形化
- 支持筛选: author, date range, path
- 选中 commit 展开详细 diff

### 4.3 Stash/Shelve Panel (新面板)
- Stash 列表 + 操作 (pop, apply, drop, branch)
- Shelve 列表 + 操作 (restore, apply, delete)

### 4.4 命令面板增强
- `:` 命令模式支持: `:w` (commit), `:q` (quit), `:help`, `:branch`, `:checkout`, `:fetch`, `:pull`, `:push`, `:rebase`, `:merge`, `:stash`, `:shelve`, `:log`, `:ignore`

### 4.5 Help Panel 增强
- 完整的 vim 键位说明
- JetBrains 风格操作指南
- 面板特定的键位说明 (根据当前焦点面板动态显示)

---

## Phase 5: 测试脚本

### 5.1 单元测试 (每个 gitops 模块)
- `tests/gitops_status.rs` — 状态读取测试
- `tests/gitops_index.rs` — 暂存操作测试
- `tests/gitops_commit.rs` — 提交测试
- `tests/gitops_branch.rs` — 分支操作测试
- `tests/gitops_remote.rs` — 远程操作测试 (mock)
- `tests/gitops_stash.rs` — 暂存栈测试
- `tests/gitops_shelve.rs` — 搁置测试
- `tests/gitops_merge_rebase.rs` — 合并/变基测试
- `tests/gitops_ignore.rs` — ignore 管理测试
- 每个测试创建临时 git 仓库 (`tempfile::tempdir()` + `git init`)

### 5.2 集成测试脚本
- `tests/integration.sh` — shell 脚本自动化:
  1. 创建临时 git 仓库
  2. 编译 cogit
  3. 运行 cogit 注入按键序列
  4. 验证 git 状态

### 5.3 Cargo.toml 添加测试依赖
```toml
[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

---

## 执行策略

每个 Phase 拆为独立的 opencode 任务，每个任务完成后验证 `cargo check` + `cargo test`。

1. Phase 1 (基础设施) — 3-4 个 opencode 任务
2. Phase 2 (Vim 键位) — 2 个 opencode 任务
3. Phase 3 (JetBrains 功能) — 3-4 个 opencode 任务
4. Phase 4 (UI 面板) — 2-3 个 opencode 任务
5. Phase 5 (测试) — 2 个 opencode 任务
