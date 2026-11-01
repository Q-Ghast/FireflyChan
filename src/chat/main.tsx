import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ChatPanel from "../overlay/ChatPanel";
import { initTheme } from "../shared/theme";
import type { ConversationViewDto } from "../shared/types";
import "../shared/base.css";
import "./chat.css";

function App() {
  const [conversation, setConversation] =
    useState<ConversationViewDto | null>(null);

  useEffect(() => {
    void initTheme();
    void invoke<ConversationViewDto>("conversation_get").then(setConversation);
  }, []);

  if (!conversation) {
    return null;
  }

  return (
    <div className="standalone-root">
      <ChatPanel
        conversation={conversation}
        standalone
        onClose={() => {
          void getCurrentWindow().close();
        }}
      />
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
