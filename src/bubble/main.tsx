import React, { useEffect, useRef, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import firefly from "../assets/firefly-widget.png";
import { initTheme } from "../shared/theme";
import type { BubbleTextPayload } from "../shared/types";
import "../shared/base.css";
import "./bubble.css";

function toPlainText(text: string) {
  return text
    .replace(/```[\s\S]*?```/g, "［代码片段］")
    .replace(/!\[[^\]]*\]\([^)]*\)/g, "")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/^#{1,6}\s+/gm, "")
    .replace(/\*\*|__|`|~~/g, "")
    .replace(/^\s*[-+]\s+/gm, "· ")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

function App() {
  const [text, setText] = useState("正在思考…");
  const [side, setSide] = useState<"left" | "right">("left");
  const bodyRef = useRef<HTMLDivElement>(null);
  const hideTimer = useRef<number | null>(null);

  function clearHideTimer() {
    if (hideTimer.current !== null) {
      window.clearTimeout(hideTimer.current);
      hideTimer.current = null;
    }
  }

  function scheduleHide(delay = 12000) {
    clearHideTimer();
    hideTimer.current = window.setTimeout(() => {
      void invoke("bubble_hide");
    }, delay);
  }

  function dismiss() {
    clearHideTimer();
    void invoke("bubble_hide");
  }

  useEffect(() => {
    void initTheme();
    const unlisten = listen<BubbleTextPayload>("bubble-text", (event) => {
      const value = toPlainText(event.payload.text || "正在思考…");
      setText(value.length > 1200 ? `${value.slice(0, 1200)}…` : value);
      setSide(event.payload.side === "right" ? "right" : "left");
      scheduleHide();
    });

    return () => {
      void unlisten.then((fn) => fn());
      clearHideTimer();
    };
  }, []);

  useEffect(() => {
    const el = bodyRef.current;
    if (!el) {
      return;
    }
    const observer = new ResizeObserver(() => {
      void invoke("bubble_resize", { height: el.scrollHeight });
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <div
      className={`bubble-wrap side-${side}`}
      title="点击关闭"
      onClick={dismiss}
      onMouseEnter={clearHideTimer}
      onMouseLeave={() => scheduleHide(4000)}
    >
      <div className="bubble" ref={bodyRef}>
        <div className="bubble-title">
          <img className="bubble-avatar" src={firefly} alt="" />
          <span>流萤</span>
          <span className="bubble-tag">Firefly</span>
        </div>
        <div className="bubble-text">{text}</div>
      </div>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
