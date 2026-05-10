use regex::Regex;
use reqwest::blocking::Client;
use std::time::Duration;

use super::ToolResult;

pub fn web_fetch(url: &str, max_length: usize) -> ToolResult {
    let client = match Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("TyCode/1.0 (AI Agent; +https://github.com/tycode)")
        .build()
    {
        Ok(c) => c,
        Err(e) => return ToolResult::err(format!("Failed to build HTTP client: {e}")),
    };

    let response = match client.get(url).send() {
        Ok(r) => r,
        Err(e) => return ToolResult::err(format!("Request failed: {e}")),
    };

    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let body = match response.text() {
        Ok(b) => b,
        Err(e) => return ToolResult::err(format!("Failed to read response: {e}")),
    };

    if !status.is_success() {
        return ToolResult::err(format!("HTTP {status}: {}", &body[..body.len().min(500)]));
    }

    let text = if content_type.contains("text/html") || content_type.is_empty() {
        html_to_text(&body)
    } else if content_type.contains("application/json") {
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or(body),
            Err(_) => body,
        }
    } else {
        body
    };

    let trimmed = text.trim().to_string();
    let max = max_length.max(1000);
    if trimmed.len() > max {
        let mut cut = max;
        while !trimmed.is_char_boundary(cut) { cut -= 1; }
        ToolResult::ok(format!("{}\n\n[Truncated — {} chars total]", &trimmed[..cut], trimmed.len()))
    } else {
        ToolResult::ok(trimmed)
    }
}

fn html_to_text(html: &str) -> String {
    // Remove script and style blocks
    let re_script = Regex::new(r"(?si)<script[^>]*>.*?</script>").unwrap();
    let re_style  = Regex::new(r"(?si)<style[^>]*>.*?</style>").unwrap();
    let re_head   = Regex::new(r"(?si)<head[^>]*>.*?</head>").unwrap();

    let s = re_head.replace_all(html, "");
    let s = re_script.replace_all(&s, "");
    let s = re_style.replace_all(&s, "");

    // Block-level tags → newlines
    let re_block = Regex::new(r"(?i)</?(?:br|p|div|li|tr|h[1-6]|article|section|header|footer|nav|main|aside)[^>]*>").unwrap();
    let s = re_block.replace_all(&s, "\n");

    // Strip all remaining tags
    let re_tags = Regex::new(r"<[^>]+>").unwrap();
    let s = re_tags.replace_all(&s, "");

    // Decode HTML entities
    let s = s
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&hellip;", "…");

    // Collapse multiple blank lines
    let re_blank = Regex::new(r"\n{3,}").unwrap();
    let s = re_blank.replace_all(&s, "\n\n");

    // Trim leading/trailing whitespace per line
    s.lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join("\n")
}
