# FireflyChan

[English](README.md) | **简体中文**

FireflyChan 是一个常驻 Windows 桌面的 AI 截图助手，主题角色是《崩坏：星穹铁道》中的**流萤**。按下快捷键冻结屏幕、框选一块区域，就可以和住在桌面边缘的流萤聊聊它。

当前版本：**v1.0.0**

## 功能

- **桌面挂件**：透明、置顶、可自由拖动，位置会自动保存；支持右键菜单和点击摸头的 Q 弹反馈。
- **Snipaste 式截图**：`Alt + Q` 打开冻结屏幕的遮罩，拖动框选，`Enter` 确认，`Esc` 取消。遮罩会预创建并复用，避免每次截图的白闪和 WebView 启动延迟。
- **聊天面板**：左侧面板流式显示 DeepSeek 回复，支持 Markdown、停止生成、错误提示；对话会持续保留，只有点「新对话」才清空。
- **气泡**：回复同步显示在挂件旁的气泡里；鼠标悬停时暂停自动消失，点击可关闭。
- **流萤人格**：克制、温柔、句子偏短。称呼你为 **Qxiane**，不说漂亮话，不写括号动作或长段大道理；需要详细解释时会展开。
- **联网搜索**：聊天输入框上方有「联网」开关；开启后会获取 Bing RSS 搜索结果，连同来源链接一起交给模型。
- **快捷提示词**：在设置中最多配置 9 条，用 `Alt + 1` … `Alt + 9` 快速填入输入框。
- **直接聊天**：`Alt + C` 不截图，直接打开独立聊天窗口。
- **Win11 风格界面**：支持浅色 / 深色 / 跟随系统，亚克力质感面板、无边框设置窗口和统一蓝绿色主题。
- **安全存储**：DeepSeek API Key 保存在 Windows 凭据管理器，不写入仓库或配置文件。

## 安装

安装包位于 [`installers/`](installers/)：

```text
installers/FireflyChan_1.0.0_x64-setup.exe
```

系统要求：Windows 10/11 x64。NSIS 安装包为当前用户安装，并内嵌 WebView2 Bootstrapper。

> 当前安装包**没有代码签名**，Windows SmartScreen 可能提示“未知发布者”。这是签名限制，不是安装包损坏。

## 使用

| 操作 | 快捷键 / 手势 |
| --- | --- |
| 打开截图遮罩 | `Alt + Q` |
| 直接打开聊天 | `Alt + C` |
| 取消截图 / 关闭当前遮罩 | `Esc` |
| 确认框选 | `Enter` |
| 填入第 1–9 条快捷提示词 | `Alt + 1` … `Alt + 9` |
| 发送消息 | 输入框中按 `Enter` |
| 换行 | `Shift + Enter` |
| 移动挂件 | 拖动挂件 |
| 摸头 | 单击挂件（不拖动） |
| 挂件菜单 | 右键挂件 |
| 新对话 | 聊天标题栏的「新对话」 |
| 切换主题 | 设置 → 外观 |

## 开发

前置条件：

- Windows 10/11 x64 和 WebView2 Runtime
- Node.js 20.19+ 或 22.12+、pnpm
- Rust stable MSVC 工具链
- Visual Studio C++ Build Tools 和 Windows SDK

```powershell
pnpm install
pnpm dev          # Tauri 开发窗口
pnpm dev:web      # 只运行前端
pnpm build        # 发布构建，并把 NSIS 安装包复制到 installers/
```

`pnpm build` 会生成：

```text
src-tauri/target/release/bundle/nsis/FireflyChan_<version>_x64-setup.exe
```

并自动复制到 `installers/`，同时更新 `installers/SHA256.txt`。

## 数据位置

- 配置：`%APPDATA%\com.fireflychan.desktop\config.json`
- 对话：`%APPDATA%\com.fireflychan.desktop\conversation.json`
- 对话截图：`%APPDATA%\com.fireflychan.desktop\conversations\<id>\images\`
- 临时截图：`%LOCALAPPDATA%\com.fireflychan.desktop\captures\`
- API Key：Windows 凭据管理器（服务 `FireflyChan`，用户 `deepseek`）

## 技术栈

- Tauri 2 + Rust：窗口、截图、全局快捷键、流式 AI 请求、持久化
- React 19 + TypeScript + Vite：挂件、截图遮罩、气泡、独立聊天、设置
- `xcap` 负责屏幕捕获，`reqwest` + native TLS 处理 DeepSeek SSE，`keyring` 保存 Windows 凭据

## 目录

```text
src/                    React 界面（widget / overlay / bubble / chat / settings）
src/overlay/ChatPanel.tsx  共用聊天面板
src/assets/             挂件图片和聊天头像
src-tauri/src/          Rust 后端（窗口、截图、AI、配置、持久化）
scripts/                构建和美术辅助脚本
installers/             提交到仓库的安装包和校验文件
docs/                   验证报告和发布记录
```

## 已知限制

- 安装包暂未代码签名。
- 尚未接入 TTS / 语音播放。
- 多显示器混合 DPI 还未在硬件上充分验证。
- 聊天暂不支持拖拽上传文件。
- 联网搜索需要网络和 Windows 的 `curl.exe`；搜索失败时会自动继续普通回答。
- 暂未实现开机自启和历史对话列表。

## 许可证

[MIT](LICENSE)
