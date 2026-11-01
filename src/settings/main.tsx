import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { PublicConfigDto } from "../shared/types";
import { initTheme, type ThemeMode } from "../shared/theme";
import firefly from "../assets/firefly-widget.png";
import "../shared/base.css";
import "./settings.css";

const THEME_LABELS: Record<ThemeMode, string> = {
  system: "跟随系统",
  light: "浅色",
  dark: "深色",
};

function normalizePrompts(values: string[] | undefined): string[] {
  const normalized = (values ?? []).slice(0, 9).map((value) => value ?? "");
  while (normalized.length < 9) {
    normalized.push("");
  }
  return normalized;
}

function App() {
  const [baseUrl, setBaseUrl] = useState("https://api.deepseek.com");
  const [model, setModel] = useState("deepseek-flash");
  const [apiKey, setApiKey] = useState("");
  const [hasKey, setHasKey] = useState(false);
  const [theme, setTheme] = useState<ThemeMode>("system");
  const [prompts, setPrompts] = useState<string[]>(normalizePrompts([]));
  const [status, setStatus] = useState("正在读取配置…");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void initTheme();
    void (async () => {
      try {
        const config = await invoke<PublicConfigDto>("config_get");
        setBaseUrl(config.baseUrl);
        setModel(config.model);
        setHasKey(config.hasApiKey);
        setTheme(config.theme);
        setPrompts(normalizePrompts(config.quickPrompts));
        setStatus(config.hasApiKey ? "已检测到已保存的 API Key。" : "还没有配置 API Key。");
      } catch (error) {
        setStatus(`读取配置失败：${String(error)}`);
      }
    })();
  }, []);

  async function changeTheme(next: ThemeMode) {
    setTheme(next);
    try {
      const config = await invoke<PublicConfigDto>("theme_set", { theme: next });
      setTheme(config.theme);
    } catch (error) {
      setStatus(`切换主题失败：${String(error)}`);
    }
  }

  function setPrompt(index: number, value: string) {
    setPrompts((previous) => {
      const next = [...previous];
      next[index] = value;
      return next;
    });
  }

  async function save() {
    setBusy(true);
    setStatus("正在保存…");
    try {
      const config = await invoke<PublicConfigDto>("config_save", {
        apiKey: apiKey.trim() ? apiKey.trim() : null,
        baseUrl,
        model,
        theme,
        quickPrompts: normalizePrompts(prompts),
      });
      setBaseUrl(config.baseUrl);
      setModel(config.model);
      setHasKey(config.hasApiKey);
      setTheme(config.theme);
      setPrompts(normalizePrompts(config.quickPrompts));
      setApiKey("");
      setStatus("已保存。可以点“测试连接”确认 Key 是否可用。");
    } catch (error) {
      setStatus(`保存失败：${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function test() {
    setBusy(true);
    setStatus("正在连接 DeepSeek…");
    try {
      const message = await invoke<string>("config_test");
      setStatus(message);
    } catch (error) {
      setStatus(`连接失败：${String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  async function capture() {
    try {
      await invoke("session_start");
      await getCurrentWindow().close();
    } catch (error) {
      setStatus(`启动截图失败：${String(error)}`);
    }
  }

  return (
    <div className="settings-shell">
      <div
        className="window-titlebar"
        onPointerDown={(event) => {
          if (event.button === 0) {
            void invoke("window_drag");
          }
        }}
      >
        <span className="window-title">设置</span>
        <div className="window-controls">
          <button
            className="window-control"
            title="最小化"
            onPointerDown={(event) => event.stopPropagation()}
            onClick={() => void getCurrentWindow().minimize()}
          >
            <span className="minimize-glyph" />
          </button>
          <button
            className="window-control close"
            title="关闭"
            onPointerDown={(event) => {
              event.stopPropagation();
              void getCurrentWindow().close();
            }}
          >
            ×
          </button>
        </div>
      </div>

      <div className="ambient ambient-a" />
      <div className="ambient ambient-b" />

      <header className="brand">
        <div className="brand-avatar">
          <img src={firefly} alt="" />
        </div>
        <div>
          <h1>流萤</h1>
          <p>FireflyChan · 桌面截图助手</p>
        </div>
        <span className={`key-state ${hasKey ? "ready" : ""}`}>
          {hasKey ? "已连接" : "待配置"}
        </span>
      </header>

      <main>
        <section className="card">
          <div className="card-head">
            <h2>外观</h2>
            <span className="card-tag">Acrylic</span>
          </div>
          <div className="row theme-row">
            <span>主题模式</span>
            <div className="theme-segment" role="group" aria-label="主题模式">
              {(["system", "light", "dark"] as ThemeMode[]).map((mode) => (
                <button
                  key={mode}
                  className={theme === mode ? "active" : ""}
                  onClick={() => void changeTheme(mode)}
                >
                  {THEME_LABELS[mode]}
                </button>
              ))}
            </div>
          </div>
          <p className="hint">
            默认跟随 Windows 的深色/浅色设置；也可以强制使用浅色或深色。
          </p>
        </section>

        <section className="card">
          <div className="card-head">
            <h2>快捷提示词</h2>
            <span className="card-tag">Alt + 1–9</span>
          </div>
          <p className="hint">
            在聊天输入框按 Alt + 数字键，快速把对应提示词填入对话框。留空的槽位不会生效。
          </p>
          <div className="prompt-grid">
            {prompts.map((prompt, index) => (
              <label className="prompt-slot" key={index}>
                <span>Alt+{index + 1}</span>
                <input
                  value={prompt}
                  maxLength={500}
                  placeholder={`提示词 ${index + 1}`}
                  onChange={(event) => setPrompt(index, event.target.value)}
                />
              </label>
            ))}
          </div>
          <p className="hint">修改后点击底部「保存配置」生效。</p>
        </section>

        <section className="card">
          <div className="card-head">
            <h2>模型连接</h2>
            <span className="card-tag">DeepSeek</span>
          </div>

          <label className="field">
            <span>API Key</span>
            <input
              type="password"
              value={apiKey}
              onChange={(event) => setApiKey(event.target.value)}
              placeholder={hasKey ? "已保存，留空则保持不变" : "sk-..."}
              autoComplete="off"
            />
          </label>
          <p className="hint">
            Key 保存在 Windows 凭据管理器，不会写进配置文件，也不会发给模型服务以外的地方。
          </p>

          <div className="field-grid">
            <label className="field">
              <span>Base URL</span>
              <input
                value={baseUrl}
                onChange={(event) => setBaseUrl(event.target.value)}
                placeholder="https://api.deepseek.com"
              />
            </label>
            <label className="field">
              <span>模型</span>
              <input
                value={model}
                onChange={(event) => setModel(event.target.value)}
                placeholder="deepseek-flash"
              />
            </label>
          </div>

          <div className="actions">
            <button className="primary" disabled={busy} onClick={() => void save()}>
              保存配置
            </button>
            <button disabled={busy || !hasKey} onClick={() => void test()}>
              测试连接
            </button>
          </div>

          <div className="status">{status}</div>
        </section>

        <section className="card compact">
          <div className="card-head">
            <h2>快捷操作</h2>
          </div>
          <div className="row">
            <span>唤起截图</span>
            <kbd>Alt + Q</kbd>
          </div>
          <div className="row">
            <span>直接打开对话</span>
            <kbd>Alt + C</kbd>
          </div>
          <div className="row">
            <span>结束当前截图会话</span>
            <kbd>Esc</kbd>
          </div>
          <button className="ghost" onClick={() => void capture()}>
            立即截一张
          </button>
        </section>

        <footer>
          流萤会记住这段对话；只有点击聊天面板里的“新对话”，才会重新开始。
        </footer>
      </main>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
