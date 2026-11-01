#[derive(Clone, Debug)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub snippet: String,
}

pub async fn web_search(query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let url = reqwest::Url::parse_with_params(
        "https://www.bing.com/search",
        &[("format", "rss"), ("mkt", "zh-CN"), ("q", query)],
    )
    .map_err(|error| error.to_string())?
    .to_string();

    let body = tokio::task::spawn_blocking(move || {
        let output = std::process::Command::new("curl.exe")
            .args([
                "-sSL",
                "--max-time",
                "15",
                "-A",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/130 Safari/537.36",
                &url,
            ])
            .output()
            .map_err(|error| format!("启动搜索失败：{error}"))?;
        if !output.status.success() {
            return Err(format!(
                "搜索请求失败：{}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    })
    .await
    .map_err(|error| error.to_string())??;

    Ok(parse_results(&body, limit))
}

fn parse_results(body: &str, limit: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();
    for item in body.split("<item>").skip(1) {
        let block = item.split("</item>").next().unwrap_or_default();
        let title = extract_tag(block, "title").unwrap_or_default();
        let link = extract_tag(block, "link").unwrap_or_default();
        let snippet = extract_tag(block, "description").unwrap_or_default();
        let title = strip_tags(&title);
        let snippet = truncate(&strip_tags(&snippet), 360);
        if title.is_empty() || link.is_empty() {
            continue;
        }
        results.push(SearchResult {
            title,
            link,
            snippet,
        });
        if results.len() >= limit {
            break;
        }
    }
    results
}

fn extract_tag(block: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = block.find(&open)? + open.len();
    let end = block[start..].find(&close)? + start;
    Some(decode_entities(&block[start..end]))
}

fn decode_entities(value: &str) -> String {
    let mut out = value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'");

    let mut result = String::with_capacity(out.len());
    let mut rest = out.as_str();
    while let Some(index) = rest.find("&#") {
        result.push_str(&rest[..index]);
        let after = &rest[index + 2..];
        let Some(end) = after.find(';') else {
            result.push_str(&rest[index..]);
            return result;
        };
        let number = &after[..end];
        if let Ok(code) = number.parse::<u32>() {
            if let Some(ch) = char::from_u32(code) {
                result.push(ch);
            }
        }
        rest = &after[end + 1..];
    }
    result.push_str(rest);
    out.clear();
    result
}

fn strip_tags(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

pub fn format_context(results: &[SearchResult]) -> String {
    results
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "[{}] {}\n{}\n来源：{}",
                index + 1,
                item.title,
                item.snippet,
                item.link
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
