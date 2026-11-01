# FireflyChan / 流萤

一个常驻 Windows 桌面的 AI 截图助手：按 `Alt + Q` 唤起类似 Snipaste 的框选界面，框选屏幕后直接在左侧和「流萤」对话，回答会同时出现在聊天面板和挂件旁的气泡里。

当前是 **M0 技术验证完成后的可交互原型**：挂件、截屏框选、裁剪、DeepSeek 流式对话、对话持久化、气泡和蓝绿色主题都已打通；安装包、文件上传、人格/TTS、多屏混合 DPI 等仍在后续计划中。

## 功能

- **桌面挂件**：透明置顶、可拖动、右键菜单（截图 / 隐藏 / 设置 / 退出），其他应用全屏时自动隐藏。
- **截图框选**：`Alt + Q` 冻结屏幕、暗化、拖拽选区，`Enter` 确认，`Esc` 取消；支持 125% DPI 下的像素级裁剪。
- **流萤对话**：左侧面板流式显示 DeepSeek 回答，支持 Markdown、代码块、停止生成、错误提示。
- **对话保留**：一次对话会跨多次截图会话保留；只有点击聊天面板的「新对话」才会清空，重新开始。
- **挂件气泡**：回答同步出现在挂件旁，自动隐藏，不抢焦点、可鼠标穿透。
- **设置**：DeepSeek API Key、Base URL、模型、连接测试；Key 存入 Windows 凭据管理器，不写进配置文件。
- **主题**：以流萤的蓝绿色为主题，挂件使用 `src/assets/firefly-widget.png`，应用/托盘图标由同一张图生成。

## 技术栈

- Tauri 2.12 + Rust（窗口、截屏、全屏检测、DeepSeek 流式请求、配置/对话持久化）
- React 19 + TypeScript + Vite（四个页面：widget / overlay / bubble / settings）
- xcap（Windows 屏幕捕获）、reqwest + native-tls（DeepSeek SSE）、keyring（Windows 凭据管理器）

## 开发运行

前置：

- Windows 10/11 x64、WebView2 Runtime
- Node.js 20.19+ / 22.12+、pnpm
- Rust stable MSVC（`rustup default stable-x86_64-pc-windows-msvc`）
- Visual Studio C++ Build Tools + Windows SDK

```powershell
pnpm install
pnpm dev
```

首次启动如果没有保存过 API Key，会自动打开设置窗口；填入 DeepSeek API Key（默认模型 `deepseek-flash`）后点「保存配置」→「测试连接」。

只看前端：

```powershell
pnpm dev:web
```

生产构建：

```powershell
pnpm build:web   # 类型检查 + 前端构建
pnpm build       # Tauri 打包（NSIS，尚未做签名）
```

## 使用

| 操作 | 说明 |
|---|---|
| `Alt + Q` | 唤起截图框选 |
| 拖拽 | 选择区域；`Enter` 确认，`Esc` 取消 |
| 聊天输入 | `Enter` 发送，`Shift + Enter` 换行 |
| `Esc` | 结束当前截图会话（对话会保留） |
| 点击「新对话」 | 清空当前对话和待发送截图 |
| 拖动挂件 | 移动位置 |
| 右键挂件 | 截图 / 隐藏挂件 / 设置 / 退出 |

## 数据位置

- 配置：`%APPDATA%\com.fireflychan.desktop\config.json`（Base URL、模型）
- 对话：`%APPDATA%\com.fireflychan.desktop\conversation.json`
- 对话截图：`%APPDATA%\com.fireflychan.desktop\conversations\<id>\images\`
- 临时抓屏：`%LOCALAPPDATA%\com.fireflychan.desktop\captures\`
- API Key：Windows 凭据管理器（服务名 `FireflyChan`，用户 `deepseek`）

## 目录

```
src/                    React 页面（widget / overlay / bubble / settings）
src/overlay/ChatPanel.tsx  对话面板
src/assets/            挂件图片
resources/             图标源文件
src-tauri/src/         Rust 后端（窗口、截屏、AI、配置、全屏检测）
scripts/prepare_art.py 挂件去背 + 应用图标生成脚本
docs/M0-REPORT.md      M0 技术验证记录
```

## 已知限制 / 下一步

- 多显示器 + 混合 DPI 尚未在硬件上实测；当前按显示器索引映射，后续改为物理几何匹配。
- 聊天暂不支持拖入/上传文件；计划在后续加入图片、文本附件。
- 人格设定、语音（TTS）和音色选择尚未开始。
- 挂件位置持久化、贴边吸附、历史对话列表、安装包签名待完善。
