# FireflyChan

**English** | [简体中文](README.zh-CN.md)

FireflyChan is a Windows desktop AI screenshot companion themed around **Firefly** from *Honkai: Star Rail*. Press a hotkey, freeze the screen, select a region, and talk about it with an AI companion that lives at the edge of your desktop.

Current release: **v1.0.0**

## Highlights

- **Desktop widget**: transparent, always-on-top, freely draggable, with persisted position, right-click menu, and a short Q-bouncy head-pat reaction on click.
- **Snipaste-style capture**: `Alt + Q` opens a frozen-screen overlay; drag to select, `Enter` to confirm, `Esc` to cancel. The overlay is pre-created and reused to avoid white flashes and repeated WebView startup.
- **Chat panel**: DeepSeek streaming replies on the left, Markdown support, stop generation, retry-friendly error banner, and one persistent conversation until you press "New chat".
- **Bubble**: replies also appear in a compact desktop bubble near the widget. It pauses auto-hide on hover and can be dismissed by clicking.
- **Persona**: a restrained, gentle Firefly voice. She calls you **Qxiane**, keeps replies short, and avoids role-play stage directions or long lectures unless you ask for detail.
- **Web search**: optional `联网` toggle in the composer. When enabled, Bing RSS results are fetched and supplied to the model with source links.
- **Quick prompts**: configure up to nine prompt slots in Settings and fill the composer with `Alt + 1` … `Alt + 9`.
- **Direct chat**: `Alt + C` opens a standalone chat window without taking a screenshot.
- **Win11-style UI**: light / dark / follow-system themes, acrylic-inspired surfaces, a frameless settings window, and consistent teal Firefly accents.
- **Secure key storage**: the DeepSeek API key is stored in Windows Credential Manager, not in the repository or config file.

## Install

Download the latest installer from [`installers/`](installers/):

```text
installers/FireflyChan_1.0.0_x64-setup.exe
```

Requirements: Windows 10/11 x64. The NSIS installer uses a per-user install and embeds the WebView2 bootstrapper.

> The current installer is **not code-signed**. Windows SmartScreen may show an "Unknown publisher" warning. This is a signing limitation, not a corrupted installer.

## Usage

| Action | Shortcut / gesture |
| --- | --- |
| Open screenshot overlay | `Alt + Q` |
| Open chat directly | `Alt + C` |
| Cancel capture / close current overlay | `Esc` |
| Confirm selection | `Enter` |
| Fill quick prompt 1–9 | `Alt + 1` … `Alt + 9` |
| Send message | `Enter` in composer |
| New line | `Shift + Enter` |
| Move widget | Drag the widget |
| Pat the widget | Click without dragging |
| Widget menu | Right-click the widget |
| New conversation | "新对话" in the chat header |
| Change theme | Settings → Appearance |

## Development

Prerequisites:

- Windows 10/11 x64 and WebView2 Runtime
- Node.js 20.19+ or 22.12+, pnpm
- Rust stable MSVC toolchain
- Visual Studio C++ Build Tools and Windows SDK

```powershell
pnpm install
pnpm dev          # Tauri development window
pnpm dev:web      # frontend only
pnpm build        # release build, then copy the NSIS installer to installers/
```

`pnpm build` produces:

```text
src-tauri/target/release/bundle/nsis/FireflyChan_<version>_x64-setup.exe
```

and automatically copies it to `installers/` with a SHA-256 entry in `installers/SHA256.txt`.

## Data locations

- Config: `%APPDATA%\com.fireflychan.desktop\config.json`
- Conversation: `%APPDATA%\com.fireflychan.desktop\conversation.json`
- Conversation images: `%APPDATA%\com.fireflychan.desktop\conversations\<id>\images\`
- Temporary captures: `%LOCALAPPDATA%\com.fireflychan.desktop\captures\`
- API key: Windows Credential Manager (service `FireflyChan`, user `deepseek`)

## Tech stack

- Tauri 2 + Rust: windows, capture, global shortcuts, streaming AI requests, persistence
- React 19 + TypeScript + Vite: widget, overlay, bubble, standalone chat, settings
- `xcap` for screen capture, `reqwest` with native TLS for DeepSeek SSE, `keyring` for Windows credentials

## Project layout

```text
src/                    React UI (widget / overlay / bubble / chat / settings)
src/overlay/ChatPanel.tsx  shared chat panel
src/assets/             widget art and chat avatar
src-tauri/src/          Rust backend (windows, capture, AI, config, persistence)
scripts/                build and artwork helpers
installers/             committed release installers + checksums
docs/                   validation and release notes
```

## Known limitations

- The installer is unsigned.
- TTS / voice playback is not implemented yet.
- Mixed-DPI multi-monitor setups have not been fully hardware-tested.
- Chat does not support drag-and-drop file uploads yet.
- Web search requires network access and the Windows `curl.exe`; if search fails, chat continues without web results.
- Windows auto-start and a history browser are not implemented yet.

## License

[MIT](LICENSE)
