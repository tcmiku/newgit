# gitpane

把 IDE 里的 Git 工作流放进一个轻量的独立窗口。打开仓库后，可以查看变更与提交图、暂存和提交文件、切换分支，并与远程仓库同步。

gitpane 使用 **Tauri 2 + Rust + Svelte 5**，调用系统 Git，复用系统 WebView。当前面向 **Windows 和 macOS**；Linux 版暂缓开发。项目仍处于早期阶段。

## 功能

- **变更与差异**：按暂存区、工作区查看文件；支持文件级暂存／取消暂存、已跟踪文本文件的代码块暂存、并排／行内差异。
- **提交历史**：图形化展示分叉、合并、本地与远程分支及标签；点击提交查看差异，也可切换到原生 `git log --graph` 视图。工作区没有变更时，完整模式默认显示提交图。
- **远程与分支**：配置远程地址，Fetch、仅快进 Pull、Push；创建／切换本地分支，首次推送时发布分支并设置 upstream。
- **Mini 模式**：将窗口缩小到 460 × 620，保留暂存、提交、拉取、推送、远程设置和分支操作。
- **集成终端**：完整模式可在仓库目录打开交互式 Shell；支持颜色和窗口尺寸变化。关闭终端面板会结束该 Shell，Mini 模式不加载终端。
- **托盘与菜单栏**：Windows／macOS 可从系统托盘重新打开窗口；设置中可选“关闭时收起到托盘”或“直接退出”，默认收起到托盘。macOS 另提供菜单栏 Mini 面板。
- **日常便利功能**：最近仓库、重启恢复、提交说明草稿、文件搜索、暗／亮主题和快捷键。仓库变更通过文件监听刷新。

### 快速上手

1. 点击“打开仓库”，选择已有 Git 仓库；也可以拖入仓库文件夹或输入路径。
2. 在“变更”中检查差异，点击 `+` 暂存文件，填写提交说明后提交。
3. 在“提交历史”查看提交图；点击提交可检查对应差异。
4. 通过顶部的远程设置添加远程仓库，再使用 Fetch、Pull 或 Push。尚无 upstream 的分支可从推送入口首次发布。
5. 点击右上角的 Mini 按钮切换小窗口。应用设置入口在完整模式左下角或 Mini 模式右上角。

完整模式左侧的终端图标可展开底部终端，也可使用 Ctrl / Cmd + ` 切换。Windows 使用 PowerShell，macOS 使用系统配置的 Shell。

应用关闭后若仍在托盘运行，左键点击托盘图标可恢复窗口；右键菜单可退出程序。

| 快捷键                 | 功能                                   |
| ---------------------- | -------------------------------------- |
| Ctrl / Cmd + O         | 打开仓库                               |
| Ctrl / Cmd + P         | 命令面板（完整模式）                   |
| Ctrl / Cmd + R         | 刷新仓库                               |
| Ctrl / Cmd + Shift + G | 查看变更                               |
| Ctrl / Cmd + Enter     | 提交暂存更改                           |
| Ctrl / Cmd + `         | 切换终端（完整模式）                   |
| Esc                    | 关闭对话框；macOS 菜单栏模式下收起面板 |

## 安装与开发

桌面版需要系统已安装 **Git**。从源码运行需要 Node.js 24、Rust stable；Windows 需要 Visual Studio C++ Build Tools 和 WebView2，macOS 需要 Xcode Command Line Tools。

```sh
npm ci
npm run desktop
```

只预览前端可运行 `npm run dev`，打开终端显示的本地地址，再点击“试用演示”。浏览器演示只读；打开真实本地仓库需要桌面版。

构建安装包：

```sh
npm run package
```

Windows 生成 NSIS 安装程序，macOS 需在 Mac 上生成 `.app` 和 `.dmg`。产物位于 `src-tauri/target/release/bundle/`；应用不会捆绑 Git。CI 在 Windows 和 macOS 上运行构建与测试，但不自动发布安装包。分发签名与 macOS 公证尚未配置。

也可以将仓库目录作为启动参数传给可执行文件，例如：

```powershell
& '.\src-tauri\target\release\gitpane.exe' 'D:\Projects\my-app'
```

## 验证

```sh
npm run build
npm test
npm run test:rust
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

`npm run build` 包含 Svelte 类型检查。Rust 测试使用临时 Git 仓库验证暂存、提交、分支、远程和冲突等流程；前端测试覆盖差异与提交图逻辑。需要试用独立仓库时，可运行 `node scripts/create-playground.mjs`，脚本会在 `.test-repos/` 下创建仓库。

## 当前边界

- Pull 只允许快进；出现分叉时不会自动合并或变基。冲突需要在外部编辑器解决，再回到 gitpane 明确暂存。
- 尚无克隆／初始化、Stash、Rebase、内置冲突编辑器、逐行暂存、图片差异及多仓库并行窗口。
- Git 认证沿用系统现有配置、SSH 和凭据工具。需要交互式认证或提交签名时，请先在终端配置。
- 非 UTF-8 文件名和非 UTF-8 文本差异尚未支持；二进制文件仍可按文件暂存。过大的文本差异会停止渲染。
- 最近仓库、提交草稿及界面偏好保存在本机 WebView 存储中。尚未给出发布版内存与启动耗时的量化数据。

## 项目结构

```text
src/App.svelte                 桌面界面与交互流程
src/components/               差异、提交图、Git 日志、Mini 模式和远程设置
src/lib/                      Tauri 调用、类型、差异与提交图逻辑
src-tauri/src/git.rs           系统 Git 适配层
src-tauri/src/lib.rs           桌面命令、文件监听与托盘行为
src-tauri/src/terminal.rs      PTY 会话与终端进程管理
scripts/                       开发辅助脚本
```

提交图交互参考 [VS Code Git Graph](https://github.com/mhutchie/vscode-git-graph)。gitpane 目前聚焦查看历史与日常提交，不包含该插件的完整高级操作集。
