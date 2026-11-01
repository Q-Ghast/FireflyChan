import React, { useEffect, useRef } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import firefly from "../assets/firefly-widget.png";
import "../shared/base.css";
import "./widget.css";

function Firefly() {
  const shellRef = useRef<HTMLDivElement>(null);
  const animationRef = useRef<Animation | null>(null);
  const startRef = useRef<{ x: number; y: number } | null>(null);
  const draggingRef = useRef(false);
  const lastPatRef = useRef(0);

  useEffect(() => {
    let timer: number | undefined;
    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .onMoved(({ payload }) => {
        if (timer) {
          window.clearTimeout(timer);
        }
        timer = window.setTimeout(() => {
          void invoke("widget_position_save", {
            x: Math.round(payload.x),
            y: Math.round(payload.y),
          });
        }, 260);
      })
      .then((dispose) => {
        unlisten = dispose;
      });

    return () => {
      if (timer) {
        window.clearTimeout(timer);
      }
      unlisten?.();
    };
  }, []);

  function pat() {
    const shell = shellRef.current;
    if (!shell) {
      return;
    }
    animationRef.current?.cancel();
    animationRef.current = shell.animate(
      [
        {
          transform: "scale3d(1, 1, 1)",
          offset: 0,
          easing: "cubic-bezier(0.2, 0.8, 0.3, 1)",
        },
        {
          transform: "scale3d(1.12, 0.82, 1)",
          offset: 0.28125,
          easing: "cubic-bezier(0.25, 0.75, 0.35, 1)",
        },
        {
          transform: "scale3d(0.96, 1.07, 1)",
          offset: 0.65625,
          easing: "cubic-bezier(0.2, 0.75, 0.3, 1)",
        },
        {
          transform: "scale3d(1.015, 0.99, 1)",
          offset: 0.84,
          easing: "ease-out",
        },
        { transform: "scale3d(1, 1, 1)", offset: 1 },
      ],
      {
        duration: 320,
        fill: "none",
      },
    );

    const now = Date.now();
    if (now - lastPatRef.current > 1200) {
      lastPatRef.current = now;
      const lines = [
        "嗯……谢谢。",
        "那个……有点突然。",
        "嘿嘿……被摸头了。",
        "我想……这样也不错。",
        "如果可以的话，再待一会儿吧。",
      ];
      const line = lines[Math.floor(Math.random() * lines.length)];
      window.setTimeout(() => {
        void invoke("bubble_show", { text: line });
      }, 220);
    }
  }

  function cleanupPointerListeners() {
    window.removeEventListener("pointermove", onWindowPointerMove);
    window.removeEventListener("pointerup", onWindowPointerUp);
    startRef.current = null;
  }

  function onWindowPointerMove(event: PointerEvent) {
    const start = startRef.current;
    if (!start || draggingRef.current) {
      return;
    }
    if (Math.hypot(event.screenX - start.x, event.screenY - start.y) < 4) {
      return;
    }
    draggingRef.current = true;
    cleanupPointerListeners();
    void invoke("widget_drag");
  }

  function onWindowPointerUp() {
    const wasDragging = draggingRef.current;
    draggingRef.current = false;
    cleanupPointerListeners();
    if (!wasDragging) {
      pat();
    }
  }

  function onPointerDown(event: React.PointerEvent<HTMLDivElement>) {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    startRef.current = { x: event.screenX, y: event.screenY };
    draggingRef.current = false;
    window.addEventListener("pointermove", onWindowPointerMove);
    window.addEventListener("pointerup", onWindowPointerUp, { once: true });
  }

  return (
    <div
      ref={shellRef}
      className="pet-shell"
      title="流萤 · 摸头 / 拖动"
      onContextMenu={(event) => {
        event.preventDefault();
        void invoke("widget_menu");
      }}
      onPointerDown={onPointerDown}
    >
      <img className="pet-image" src={firefly} alt="流萤" draggable={false} />
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Firefly />
  </React.StrictMode>,
);
