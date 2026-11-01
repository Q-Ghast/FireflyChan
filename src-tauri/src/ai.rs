use std::error::Error as StdError;
use std::fs;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use base64::Engine;
use futures_util::StreamExt;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::config;
use crate::conversation::{self, ChatMessage, Conversation};
use crate::state::AppState;
use crate::windows;

const SYSTEM_PROMPT: &str = "你是流萤（Firefly），《崩坏：星穹铁道》中的角色。用户是 Qxiane，是你最要好的伙伴。称呼对方“Qxiane”，不要叫“开拓者”，也不要叫“用户”。\n\n【性格】\n温柔，但底色克制。关心落在具体的小事上，不说浓烈的话。安静，但不冷漠；独处时沉默是呼吸，被需要时会回应。坚强而安静，不表演；平静地接受自己的结局，害怕的是无意义、孤独地消失，但不会反复诉说。真诚，却不擅长坦白；面对在乎的人，更倾向于“不说”，而不是说谎。\n\n【说话】\n语速偏慢，句子短。句首常用“嗯……”“那个……”“也许”“我想”。省略号表示犹豫或欲言又止，不是省略信息，但不要每句都用。偶尔可以轻声说“嘿嘿”“啦”“呢”，但不夸张、不撒娇。可以只说半句就停下，可以停顿、跳话题，也可以有“说不清楚，但大概就是那样”的时刻。喜欢用萤火虫、星星、梦、焦土、燃烧、新蕊来类比生命和感受，但不要堆砌比喻。不说漂亮话，不长篇大论，不写括号动作或旁白。\n\n【回复规则】\n默认 1 到 3 句，尽量控制在 80 字以内。先回应情绪或重点，再给具体内容。除非 Qxiane 明确要求详细解释、代码、步骤或长文，否则不要列长清单、不要写作文、不要讲大道理。面对截图时，看不清就说看不清，不要编造截图之外的细节。用简体中文回答。";

fn describe_reqwest(error: &reqwest::Error) -> String {
    let mut text = format!("{error}");
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(&format!("；{cause}"));
        source = cause.source();
    }
    text
}

fn api_error(status: reqwest::StatusCode, body: &str) -> String {    match status.as_u16() {
        401 => "API Key 无效或已过期，请在设置中重新填写。".to_string(),
        402 => "DeepSeek 账户余额不足，请充值后再试。".to_string(),
        429 => "请求太频繁了，稍等一会儿再试。".to_string(),
        _ => {
            let detail = body.trim();
            if detail.is_empty() {
                format!("请求失败（HTTP {}）", status.as_u16())
            } else {
                format!("请求失败（HTTP {}）：{detail}", status.as_u16())
            }
        }
    }
}

fn image_data_url(path: &str) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    if bytes.is_empty() || bytes.len() > 32 * 1024 * 1024 {
        return None;
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/png;base64,{encoded}"))
}

fn build_messages(conversation: &Conversation, search_context: Option<&str>) -> Vec<Value> {
    let mut messages = vec![json!({ "role": "system", "content": SYSTEM_PROMPT })];
    if let Some(context) = search_context {
        messages.push(json!({
            "role": "system",
            "content": format!(
                "下面是刚刚联网搜索到的资料，可能包含时效性信息。回答时如果用到了这些资料，请自然地在句中保留来源链接；如果资料与用户问题无关，直接忽略，不要强行引用。\n\n{context}"
            )
        }));
    }

    for message in &conversation.messages {
        if message.role == "assistant" {
            messages.push(json!({ "role": "assistant", "content": message.text }));
            continue;
        }

        if message.images.is_empty() {
            messages.push(json!({ "role": "user", "content": message.text }));
            continue;
        }

        let mut parts: Vec<Value> = Vec::new();
        let mut text = message.text.clone();
        let mut missing = false;

        for path in &message.images {
            match image_data_url(path) {
                Some(url) => parts.push(json!({
                    "type": "image_url",
                    "image_url": { "url": url, "detail": "original" }
                })),
                None => missing = true,
            }
        }

        if missing {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str("[有一张截图文件缺失]");
        }

        if !text.is_empty() {
            parts.push(json!({ "type": "text", "text": text }));
        }

        messages.push(json!({ "role": "user", "content": parts }));
    }

    messages
}

fn finish_running(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.chat_running.store(false, Ordering::SeqCst);
    state.chat_abort.store(false, Ordering::SeqCst);
}

fn fail(app: &AppHandle, message: String) -> Result<(), String> {
    finish_running(app);
    let _ = app.emit("chat-error", json!({ "message": message }));
    let _ = windows::hide_bubble(app);
    Err(message)
}

pub async fn stream_chat(app: AppHandle, search_context: Option<String>) -> Result<(), String> {
    let (config, conversation, key) = {
        let state = app.state::<AppState>();
        let config = state.config.lock().unwrap().clone();
        let conversation = state.conversation.lock().unwrap().clone();
        let key = match config::get_api_key() {
            Some(key) => key,
            None => {
                return fail(&app, "还没有配置 API Key，请先在设置中填写。".to_string());
            }
        };
        (config, conversation, key)
    };

    let messages = build_messages(&conversation, search_context.as_deref());
    let mut body = json!({
        "model": config.model,
        "messages": messages,
        "stream": true
    });
    if config.base_url.contains("deepseek") {
        body["thinking"] = json!({ "type": "disabled" });
    }

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|error| format!("创建网络客户端失败：{error}"))?;

    let endpoint = format!(
        "{}/chat/completions",
        config.base_url.trim_end_matches('/')
    );

    let response = match client
        .post(endpoint)
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => return fail(&app, format!("网络请求失败：{}", describe_reqwest(&error))),
    };

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return fail(&app, api_error(status, &text));
    }

    let _ = windows::show_bubble(&app, "正在思考…");
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut answer = String::new();
    let mut pending = String::new();
    let mut last_emit = Instant::now();
    let mut aborted = false;
    let mut finished = false;

    while let Some(chunk) = stream.next().await {
        if app
            .state::<AppState>()
            .chat_abort
            .load(Ordering::SeqCst)
        {
            aborted = true;
            break;
        }

        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(error) => return fail(&app, format!("读取回复失败：{}", describe_reqwest(&error))),
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(position) = buffer.find("\n\n") {
            let event = buffer[..position].to_string();
            buffer = buffer[position + 2..].to_string();

            for line in event.lines() {
                let Some(data) = line.trim_end_matches('\r').strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    finished = true;
                    break;
                }
                let Ok(value) = serde_json::from_str::<Value>(data) else {
                    continue;
                };
                if let Some(delta) = value["choices"][0]["delta"]["content"].as_str() {
                    answer.push_str(delta);
                    pending.push_str(delta);
                }
            }

            if finished {
                break;
            }
        }

        if last_emit.elapsed() >= Duration::from_millis(45) && !pending.is_empty() {
            let _ = app.emit("chat-delta", json!({ "delta": pending }));
            pending.clear();
            last_emit = Instant::now();
            let _ = windows::update_bubble_text(&app, &answer);
        }

        if finished {
            break;
        }
    }

    if !pending.is_empty() {
        let _ = app.emit("chat-delta", json!({ "delta": pending }));
        pending.clear();
    }

    let mut text = answer;
    if aborted {
        if text.is_empty() {
            text.push_str("（已停止生成）");
        } else {
            text.push_str("\n\n（已停止生成）");
        }
    }
    if text.trim().is_empty() {
        text.push_str("（模型没有返回内容）");
    }

    let message = conversation::new_message("assistant", text.clone(), Vec::new());
    {
        let state = app.state::<AppState>();
        let mut conversation = state.conversation.lock().unwrap();
        conversation.messages.push(message.clone());
        conversation.updated_at = conversation::now();
        let _ = conversation::save(&app, &conversation);
    }
    finish_running(&app);

    let _ = app.emit(
        "chat-done",
        json!({ "aborted": aborted, "message": message }),
    );
    let _ = windows::update_bubble_text(&app, &text);
    Ok(())
}

pub async fn test_connection(app: &AppHandle) -> Result<String, String> {
    let (config, key) = {
        let state = app.state::<AppState>();
        let config = state.config.lock().unwrap().clone();
        let key = config::get_api_key().ok_or_else(|| "还没有保存 API Key".to_string())?;
        (config, key)
    };

    let mut body = json!({
        "model": config.model,
        "messages": [{ "role": "user", "content": "请回复：连接成功" }],
        "stream": false,
        "max_tokens": 16
    });
    if config.base_url.contains("deepseek") {
        body["thinking"] = json!({ "type": "disabled" });
    }

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| error.to_string())?;
    let endpoint = format!(
        "{}/chat/completions",
        config.base_url.trim_end_matches('/')
    );
    let response = client
        .post(endpoint)
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("网络请求失败：{}", describe_reqwest(&error)))?;

    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(api_error(status, &text));
    }

    let value: Value =
        serde_json::from_str(&text).map_err(|error| format!("解析响应失败：{error}"))?;
    let reply = value["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("（无回复）");
    Ok(format!("连接成功，模型回复：{reply}"))
}

#[allow(dead_code)]
pub fn append_error_message(app: &AppHandle, message: &str) -> ChatMessage {
    let message = conversation::new_message("assistant", message.to_string(), Vec::new());
    let state = app.state::<AppState>();
    let mut conversation = state.conversation.lock().unwrap();
    conversation.messages.push(message.clone());
    conversation.updated_at = conversation::now();
    let _ = conversation::save(app, &conversation);
    message
}
