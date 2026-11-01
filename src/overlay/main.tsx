import React, { useCallback, useEffect, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ChatPanel from "./ChatPanel";
import { initTheme } from "../shared/theme";
import type {
  MonitorCaptureDto,
  RectDto,
  SessionChatDto,
} from "../shared/types";
import "../shared/base.css";
import "./overlay.css";

const windowLabel = getCurrentWindow().label;
const monitorIndex = Number(windowLabel.replace("overlay-", "")) || 0;

function normalizeRect(
  a: { x: number; y: number },
  b: { x: number; y: number },
): RectDto {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  return {
    x,
    y,
    width: Math.abs(a.x - b.x),
    height: Math.abs(a.y - b.y),
  };
}

function App() {
  const [monitor, setMonitor] = useState<MonitorCaptureDto | null>(null);
  const [start, setStart] = useState<{ x: number; y: number } | null>(null);
  const [current, setCurrent] = useState<{ x: number; y: number } | null>(null);
  const [session, setSession] = useState<SessionChatDto | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const draggingRef = useRef(false);
  const readyRef = useRef(false);

  function signalReady() {
    if (readyRef.current) {
      return;
    }
    readyRef.current = true;
    void invoke("overlay_ready");
  }

  useEffect(() => {
    void initTheme();
  }, []);

  useEffect(() => {
    let alive = true;
    const load = async () => {
      for (let attempt = 0; attempt < 40 && alive; attempt += 1) {
        const info = await invoke<MonitorCaptureDto | null>("overlay_info", {
          index: monitorIndex,
        });
        if (info) {
          setMonitor(info);
          return;
        }
        await new Promise((resolve) => setTimeout(resolve, 50));
      }
    };
    void load();
    return () => {
      alive = false;
    };
  }, []);

  useEffect(() => {
    const unlisten = [
      listen<MonitorCaptureDto>("overlay-update", (event) => {
        readyRef.current = false;
        setMonitor(event.payload);
        setSession(null);
        setStart(null);
        setCurrent(null);
        setBusy(false);
        setError(null);
      }),
      listen("overlay-reset", () => {
        readyRef.current = false;
        setMonitor(null);
        setSession(null);
        setStart(null);
        setCurrent(null);
        setBusy(false);
        setError(null);
      }),
    ];
    return () => {
      unlisten.forEach((promise) => {
        void promise.then((fn) => fn());
      });
    };
  }, []);

  useEffect(() => {
    if (!monitor) {
      return;
    }
    const timer = window.setTimeout(signalReady, 1500);
    return () => window.clearTimeout(timer);
  }, [monitor]);

  const cancel = useCallback(() => {
    void invoke("session_cancel");
  }, []);

  const raiseSoon = useCallback(() => {
    window.setTimeout(() => {
      void invoke("raise_companions");
    }, 50);
    window.setTimeout(() => {
      void invoke("raise_companions");
    }, 180);
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (session) {
        return;
      }
      if (event.key === "Escape") {
        if (start) {
          setStart(null);
          setCurrent(null);
        } else {
          cancel();
        }
      }
      if (event.key === "Enter" && start && current && !busy) {
        void confirm();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  const onPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (session) {
      raiseSoon();
      return;
    }
    if (busy || event.button !== 0) {
      return;
    }
    if ((event.target as HTMLElement).closest(".toolbar")) {
      return;
    }
    event.preventDefault();
    draggingRef.current = true;
    setStart({ x: event.clientX, y: event.clientY });
    setCurrent({ x: event.clientX, y: event.clientY });
  };

  useEffect(() => {
    const onMove = (event: PointerEvent) => {
      if (!draggingRef.current) {
        return;
      }
      setCurrent({ x: event.clientX, y: event.clientY });
    };
    const onUp = (event: PointerEvent) => {
      if (draggingRef.current) {
        setCurrent({ x: event.clientX, y: event.clientY });
      }
      draggingRef.current = false;
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
  }, []);

  const rect = start && current ? normalizeRect(start, current) : null;
  const selectionReady = rect !== null && rect.width > 8 && rect.height > 8;

  async function confirm() {
    if (!rect || !selectionReady || busy) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const result = await invoke<SessionChatDto>("session_confirm", {
        monitorIndex,
        rect,
      });
      setSession(result);
      setStart(null);
      setCurrent(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(false);
    }
  }

  const imageSrc = monitor ? convertFileSrc(monitor.path) : null;

  return (
    <div
      className="stage"
      onPointerDown={onPointerDown}
      style={{ cursor: session ? "default" : "crosshair" }}
    >
      {imageSrc && (
        <img
          className="shot"
          src={imageSrc}
          alt=""
          draggable={false}
          onLoad={signalReady}
          onError={signalReady}
        />
      )}

      {!session && (
        <>
          {!selectionReady && <div className="dim" />}

          {rect && (
            <div
              className="selection"
              style={{
                left: rect.x,
                top: rect.y,
                width: rect.width,
                height: rect.height,
              }}
            >
              <span className="size-badge">
                {Math.round(rect.width)} × {Math.round(rect.height)}
              </span>
              <span className="corner corner-tl" />
              <span className="corner corner-tr" />
              <span className="corner corner-bl" />
              <span className="corner corner-br" />
            </div>
          )}

          {!selectionReady && (
            <div className="hint">
              <span className="hint-dot" />
              拖动选择区域 · Esc 取消
            </div>
          )}

          {selectionReady && (
            <div
              className="toolbar"
              style={{
                left: Math.min(rect!.x + rect!.width + 12, window.innerWidth - 230),
                top: Math.min(rect!.y + rect!.height + 12, window.innerHeight - 56),
              }}
            >
              <button
                className="primary"
                onClick={() => void confirm()}
                disabled={busy}
              >
                {busy ? "处理中…" : "确认 Enter"}
              </button>
              <button
                onClick={() => {
                  setStart(null);
                  setCurrent(null);
                }}
              >
                重选
              </button>
              <button className="ghost" onClick={cancel}>
                取消 Esc
              </button>
            </div>
          )}
        </>
      )}

      {session && (
        <>
          <div className="chat-backdrop" />
          <ChatPanel
            conversation={session.conversation}
            onClose={cancel}
          />
        </>
      )}

      {error && !session && (
        <div className="error-toast" onClick={() => setError(null)}>
          {error}
        </div>
      )}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
