# GitPane

把 IDE 中熟悉的 Git 工作流，放进一个轻量独立窗口。

面向 **Windows 和 macOS** 的早期桌面原型，使用 Tauri 2、Rust、Svelte 5 和系统 Git。没有账户系统、遥测、远程前端或常驻 Node 服务。Linux 暂不作为开发与验证目标。

## 运行

开发环境需要 Node.js 24、Rust stable 和 Git。Windows 还需要 Visual Studio C++ Build Tools 及 WebView2；macOS 需要 Xcode Command Line Tools。

```sh
npm ci
npm run desktop
```

只查看前端：

```sh
npm run dev
```

打开终端输出的本地地址，点击「先看看界面」。浏览器演示为只读，不能操作本地 Git。桌面应用才提供真实仓库访问。

## 当前功能

- 打开本地仓库：文件夹选择、手动路径、拖放、最近仓库、重启恢复。
- 更改与暂存区分组、文件搜索、状态标记。
- 并排／行内文本差异、行号、基础语法着色、只渲染可见差异行。
- 文件级暂存／取消暂存、全部暂存／取消暂存。
- 已跟踪文本文件的代码块暂存；操作前校验完整差异，拒绝过期内容。
- 只提交暂存内容，按仓库保存提交说明草稿。
- 当前分支提交历史、分页加载、提交差异。
- 本地分支列表、创建与切换。
- Fetch、仅快进 Pull、Push，显示已知的领先／落后数量。
- 冲突提示；在外部编辑器解决后，逐个暂存解决结果。
- 工作区文件监听、事件合并刷新、切回窗口刷新、手动刷新。
- 命令面板、暗／亮主题、键盘快捷键。
- 正常支持 linked worktree 的仓库路径。

| 快捷键 | 功能 |
| --- | --- |
| Ctrl / Cmd + O | 打开仓库 |
| Ctrl / Cmd + P | 命令面板 |
| Ctrl / Cmd + R | 刷新仓库 |
| Ctrl / Cmd + Shift + G | 查看变更 |
| Ctrl / Cmd + Enter | 提交暂存更改 |
| Esc | 关闭对话框 |

可执行文件也接受一个仓库目录参数，例如 Windows 下：

```powershell
& '.\src-tauri\target\release\gitpane.exe' 'D:\Projects\my-app'
```

## 构建

```sh
npm run package
```

Windows 生成 NSIS 安装程序；macOS 在 Mac 上生成 `.app` 和 `.dmg`。产物在 `src-tauri/target/release/bundle/`。应用自身不捆绑 Git；Windows 安装程序可引导安装缺失的 WebView2。

跨平台 CI 位于 `.github/workflows/desktop.yml`，在 Windows / macOS 分别运行检查、测试和可执行文件编译，不会自动发布。当前环境仅验证 Windows 构建，macOS 尚未实机验证。分发签名及 macOS 公证尚未配置。

## 测试

```sh
npm run build
npm test
npm run test:rust
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

Rust 集成测试创建独立临时仓库，覆盖暂存与提交语义、首次提交前取消暂存、特殊文件名、重命名、代码块暂存和过期差异、冲突、worktree、分支以及大文件／二进制处理。前端单元测试验证差异行号与并排对齐。未进行电脑界面操作验收。

如需一个可以自由试用的真实仓库：

```sh
node scripts/create-playground.mjs
```

该命令在 `.test-repos/` 下创建一个新的独立仓库，并输出路径，不修改已有项目仓库。

## 轻量化策略

- 复用系统 WebView 与 Git；生产环境不需要 Node.js。
- 只打开一个活动仓库，关闭仓库时释放文件监听器。
- 提交历史每页 60 条，差异只在选中文件时读取。
- 文件事件合并 250 ms 后刷新，后台隐藏时不触发界面刷新，无定时全量轮询。
- 差异采用可见行渲染，超出 1 MiB 暂停文本展示；Git 输出上限 8 MiB。
- 图标按需编译，演示数据独立分包，无网络字体或图片依赖。
- Git 在后台线程中运行，通过参数数组调用，不拼接 shell 命令。
- 同一应用实例内串行处理仓库读写，防止暂存与提交相互穿插。

性能目标仍需发布版实测。前端产物大小不代表安装大小、进程内存或启动耗时。

## 首版边界

目前专注「打开 → 审查 → 暂存 → 提交」闭环。尚未提供：克隆／初始化入口、逐行暂存、图片差异、内置冲突编辑器、合并／Rebase／Stash、修改上次提交、分支删除、外部编辑器定位、复杂历史图、多仓库并行与账户登录。

默认使用系统 Git 及既有配置、SSH 与凭据工具。交互式终端密码提示和凭据管理器交互被禁用；需要交互认证或签名时，请先在终端配置。不会覆盖 `core.sshCommand` 或 `GIT_SSH_COMMAND`。网络命令和 hooks 最多等待 120 秒；超时后应刷新确认实际仓库状态。Pull 仅允许快进，分叉时会报告错误，不自动进行合并或变基。

暂不支持非 UTF-8 文件名及非 UTF-8 文本差异；二进制／大文件可以按整个文件暂存。冲突文件须在编辑器中解决后由用户明确暂存，应用不会自行选择冲突一侧。

应用会记住最近仓库路径和提交草稿，它们保存在本机 WebView 存储中。远程操作仅连接仓库配置的远程地址。

## 目录

```text
src/App.svelte                  工作空间、仓库状态与操作流程
src/components/DiffView.svelte   差异可见行渲染
src/lib/diff.ts                  Git patch 解析和并排对齐
src/lib/api.ts                   Tauri 调用与本机偏好
src-tauri/src/git.rs             系统 Git 适配层
src-tauri/src/git/tests.rs       真实临时仓库集成测试
src-tauri/src/lib.rs             IPC、任务串行化、文件监听
```
