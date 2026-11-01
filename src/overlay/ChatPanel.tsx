import React, { useEffect, useRef, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import fireflyAvatar from "../assets/firefly-avatar.png";
import type {
  ChatMessageDto,
  ConversationViewDto,
  StagedImageDto,
} from "../shared/types";
import "./chat.css";

const STREAM_ID = "streaming-assistant";

interface ChatPanelProps {
  conversation: ConversationViewDto;
  onClose: () => void;
  standalone?: boolean;
}

interface ChatDeltaPayload {
  delta: string;
}

interface ChatDonePayload {
  aborted: boolean;
  message: ChatMessageDto;
}

interface ChatErrorPayload {
  message: string;
}

function TypingDots() {
  return (
    <div className="typing">
      <span />
      <span />
      <span />
    </div>
  );
}

function Message({
  message,
  streaming,
}: {
  message: ChatMessageDto;
  streaming: boolean;
}) {
  const isUser = message.role === "user";
  return (
    <div className={`message ${isUser ? "from-user" : "from-assistant"}`}>
      {!isUser && (
        <div className="assistant-avatar">
          <img src={fireflyAvatar} alt="" />
        </div>
      )}
      <div className="message-body">
        <div className="message-meta">
          <span>{isUser ? "你" : "流萤"}</span>
          {streaming && <span className="streaming-tag">正在回应</span>}
        </div>
        {message.images.length > 0 && (
          <div className="message-images">
            {message.images.map((path) => (
              <img
                key={path}
                src={convertFileSrc(path)}
                alt="截图"
                className="message-image"
              />
            ))}
          </div>
        )}
        {message.text ? (
          isUser ? (
            <div className="user-text">{message.text}</div>
          ) : (
            <div className="markdown">
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={{
                  a: ({ children }) => <span className="md-link">{children}</span>,
                }}
              >
                {message.text}
              </ReactMarkdown>
              {streaming && <span className="cursor" />}
            </div>
          )
        ) : (
          streaming && <TypingDots />
        )}
      </div>
    </div>
  );
}

function normalizePrompts(values: string[] | undefined): string[] {
  const prompts = (values ?? []).slice(0, 9).map((value) => value ?? "");
  while (prompts.length < 9) {
    prompts.push("");
  }
  return prompts;
}

export default function ChatPanel({
  conversation,
  onClose,
  standalone = false,
}: ChatPanelProps) {
  const [messages, setMessages] = useState<ChatMessageDto[]>(
    conversation.messages,
  );
  const [staged, setStaged] = useState<StagedImageDto | null>(
    conversation.stagedImage,
  );
  const [input, setInput] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [sending, setSending] = useState(false);
  const [webSearch, setWebSearch] = useState(true);
  const [searchStatus, setSearchStatus] = useState<string | null>(null);
  const [quickPrompts, setQuickPrompts] = useState<string[]>(
    normalizePrompts([]),
  );
  const [error, setError] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const stickToBottom = useRef(true);

  useEffect(() => {
    void invoke<boolean>("web_search_get").then(setWebSearch);
    void invoke<string[]>("quick_prompts_get").then((prompts) => {
      setQuickPrompts(normalizePrompts(prompts));
    });
  }, []);

  useEffect(() => {
    const unlisten = [
      listen<{ prompts: string[] }>("prompts-changed", (event) => {
        setQuickPrompts(normalizePrompts(event.payload.prompts));
      }),
      listen<ChatMessageDto>("chat-user", (event) => {
        setMessages((previous) => [
          ...previous,
          event.payload,
          {
            id: STREAM_ID,
            role: "assistant",
            text: "",
            images: [],
            createdAt: Date.now(),
          },
        ]);
        setStaged(null);
        setInput("");
        setSending(false);
        setStreaming(true);
        setError(null);
        stickToBottom.current = true;
      }),
      listen<ChatDeltaPayload>("chat-delta", (event) => {
        setSearchStatus(null);
        setMessages((previous) => {
          const index = previous.findIndex((item) => item.id === STREAM_ID);
          if (index < 0) {
            return [
              ...previous,
              {
                id: STREAM_ID,
                role: "assistant",
                text: event.payload.delta,
                images: [],
                createdAt: Date.now(),
              },
            ];
          }
          const next = [...previous];
          next[index] = {
            ...next[index],
            text: next[index].text + event.payload.delta,
          };
          return next;
        });
        setStreaming(true);
      }),
      listen<ChatDonePayload>("chat-done", (event) => {
        setMessages((previous) => {
          const index = previous.findIndex((item) => item.id === STREAM_ID);
          if (index < 0) {
            return [...previous, event.payload.message];
          }
          const next = [...previous];
          next[index] = event.payload.message;
          return next;
        });
        setStreaming(false);
        setSending(false);
        setSearchStatus(null);
        setError(null);
      }),
      listen<{ status: "searching" | "thinking" }>("chat-status", (event) => {
        setSearchStatus(
          event.payload.status === "searching"
            ? "正在联网搜索…"
            : "正在思考…",
        );
      }),
      listen<ChatErrorPayload>("chat-error", (event) => {
        setSearchStatus(null);
        setError(event.payload.message);
        setStreaming(false);
        setSending(false);
        setMessages((previous) =>
          previous.filter((item) => item.id !== STREAM_ID),
        );
      }),
    ];

    return () => {
      unlisten.forEach((promise) => {
        void promise.then((fn) => fn());
      });
    };
  }, []);

  useEffect(() => {
    const element = listRef.current;
    if (element && stickToBottom.current) {
      element.scrollTop = element.scrollHeight;
    }
  }, [messages, streaming]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  useEffect(() => {
    const onQuickPrompt = (event: KeyboardEvent) => {
      if (
        !event.altKey ||
        event.ctrlKey ||
        event.metaKey ||
        event.shiftKey
      ) {
        return;
      }
      const match =
        /^(?:Digit|Numpad)([1-9])$/.exec(event.code) ??
        /^([1-9])$/.exec(event.key);
      if (!match) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      if (streaming) {
        return;
      }
      const prompt = quickPrompts[Number(match[1]) - 1]?.trim();
      if (!prompt) {
        return;
      }
      setInput(prompt);
      requestAnimationFrame(() => {
        const textarea = textareaRef.current;
        if (textarea) {
          textarea.focus();
          const end = prompt.length;
          textarea.setSelectionRange(end, end);
        }
      });
    };
    window.addEventListener("keydown", onQuickPrompt, true);
    return () => window.removeEventListener("keydown", onQuickPrompt, true);
  }, [quickPrompts, streaming]);

  useEffect(() => {
    const onResize = () => {
      const element = textareaRef.current;
      if (!element) {
        return;
      }
      element.style.height = "auto";
      element.style.height = `${Math.min(element.scrollHeight, 120)}px`;
    };
    onResize();
  }, [input]);

  function onScroll() {
    const element = listRef.current;
    if (!element) {
      return;
    }
    stickToBottom.current =
      element.scrollHeight - element.scrollTop - element.clientHeight < 90;
  }

  async function send() {
    if (streaming || sending) {
      return;
    }
    const text = input.trim();
    if (!text && !staged) {
      return;
    }
    setSending(true);
    setError(null);
    setSearchStatus(webSearch && text ? "正在联网搜索…" : null);
    try {
      await invoke("chat_send", { text, webSearch });
    } catch (reason) {
      setError(String(reason));
      setSending(false);
    }
  }

  async function toggleWebSearch() {
    const next = !webSearch;
    setWebSearch(next);
    try {
      await invoke("web_search_set", { enabled: next });
    } catch (reason) {
      setWebSearch(!next);
      setError(String(reason));
    }
  }

  async function stop() {    try {
      await invoke("chat_abort");
    } catch {
      // The stream may already have finished.
    }
  }

  async function newConversation() {
    if (streaming) {
      return;
    }
    try {
      const next = await invoke<ConversationViewDto>("conversation_new");
      setMessages(next.messages);
      setStaged(next.stagedImage);
      setInput("");
      setError(null);
      setSending(false);
      stickToBottom.current = true;
      textareaRef.current?.focus();
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function removeStaged() {
    try {
      await invoke("staged_clear");
      setStaged(null);
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function openSettings() {
    try {
      await invoke("open_settings");
    } catch (reason) {
      setError(String(reason));
    }
  }

  function onComposerKeyDown(event: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== "Enter" || event.shiftKey) {
      return;
    }
    if (event.nativeEvent.isComposing) {
      return;
    }
    event.preventDefault();
    void send();
  }

  return (
    <section
      className={`chat-panel${standalone ? " standalone" : ""}`}
      onPointerDown={(event) => {
        event.stopPropagation();
        window.setTimeout(() => {
          void invoke("raise_companions");
        }, 50);
        window.setTimeout(() => {
          void invoke("raise_companions");
        }, 180);
      }}
    >
      <header className="chat-head">
        <div className="chat-identity">
          <img className="chat-avatar" src={fireflyAvatar} alt="" />
          <div>
            <div className="chat-name">
              流萤
              {streaming && (
                <span className="chat-status">
                  <i />
                  正在回应
                </span>
              )}
            </div>
            <div className="chat-subtitle">把这一小块屏幕交给我吧</div>
          </div>
        </div>
        <div className="chat-head-actions">
          <button
            className="new-chat"
            onClick={() => void newConversation()}
            disabled={streaming}
            title="清空当前对话，重新开始"
          >
            新对话
          </button>
          <button
            className="close-chat"
            title="结束会话"
            onPointerDown={(event) => {
              event.stopPropagation();
              onClose();
            }}
          >
            ×
          </button>
        </div>
      </header>

      <div className="chat-list" ref={listRef} onScroll={onScroll}>
        {messages.map((message) => (
          <Message
            key={message.id}
            message={message}
            streaming={streaming && message.id === STREAM_ID}
          />
        ))}

        {streaming &&
          !messages.some((message) => message.id === STREAM_ID) && (
            <div className="message from-assistant">
              <div className="assistant-avatar">
                <img src={fireflyAvatar} alt="" />
              </div>
              <div className="message-body">
                <div className="message-meta">
                  <span>流萤</span>
                  <span className="streaming-tag">正在回应</span>
                </div>
                <TypingDots />
              </div>
            </div>
          )}
      </div>

      {error && (
        <div className="chat-error">
          <span>{error}</span>
          <button onClick={() => void openSettings()}>打开设置</button>
        </div>
      )}

      <div className="composer">
        {staged && (
          <div className="staged">
            <img src={convertFileSrc(staged.path)} alt="待发送截图" />
            <div className="staged-meta">
              <span>待发送截图</span>
              <small>
                {staged.width} × {staged.height}
              </small>
            </div>
            <button
              className="staged-remove"
              onClick={() => void removeStaged()}
              title="移除截图"
            >
              ×
            </button>
          </div>
        )}

        <div className="composer-tools">
          <button
            className={`web-toggle${webSearch ? " active" : ""}`}
            onClick={() => void toggleWebSearch()}
            title="让流萤先联网搜索，再结合搜索结果回答"
          >
            {webSearch ? "联网中" : "联网"}
          </button>
          {searchStatus && <span className="search-status">{searchStatus}</span>}
        </div>

        <div className="composer-box">
          <textarea
            ref={textareaRef}
            rows={1}
            autoFocus
            value={input}
            placeholder={staged ? "想问这张截图的什么？" : "和流萤说点什么…"}
            onChange={(event) => setInput(event.target.value)}
            onKeyDown={onComposerKeyDown}
            disabled={streaming}
          />
          {streaming ? (
            <button className="send stop" onClick={() => void stop()}>
              停止
            </button>
          ) : (
            <button
              className="send"
              onClick={() => void send()}
              disabled={sending || (!input.trim() && !staged)}
            >
              {sending ? "…" : "发送"}
            </button>
          )}
        </div>
        <div className="composer-hint">
          Enter 发送 · Shift + Enter 换行 · Esc 结束会话 · 对话会自动保留
        </div>
      </div>
    </section>
  );
}
