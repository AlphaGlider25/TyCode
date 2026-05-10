use serde_json::Value;
use std::io::copy;

use super::ToolResult;

/// Make an HTTP request.
pub fn http_request(method: &str, url: &str, headers: Option<&Value>, body: &str) -> ToolResult {
    if url.is_empty() {
        return ToolResult::err("No URL provided");
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => return ToolResult::err(format!("Failed to create HTTP client: {e}")),
    };

    let method_upper = method.to_uppercase();
    let http_method = match method_upper.as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        "HEAD" => reqwest::Method::HEAD,
        "OPTIONS" => reqwest::Method::OPTIONS,
        _ => return ToolResult::err(format!("Unsupported HTTP method: {method}")),
    };

    let mut request = client.request(http_method, url);

    // Add headers
    if let Some(hdrs) = headers {
        if let Some(obj) = hdrs.as_object() {
            for (key, val) in obj {
                if let Some(v) = val.as_str() {
                    request = request.header(key, v);
                }
            }
        }
    }

    // Add body
    if !body.is_empty() {
        request = request.body(body.to_string());
    }

    match request.send() {
        Ok(response) => {
            let status = response.status();
            let headers_map: Vec<String> = response
                .headers()
                .iter()
                .take(20)
                .map(|(k, v)| format!("  {}: {}", k, v.to_str().unwrap_or("?")))
                .collect();

            let body_text = response.text().unwrap_or_default();
            let truncated = body_text.len() > 8192;
            let body_display = if truncated {
                format!("{}...\n(truncated at 8KB)", &body_text[..8192])
            } else {
                body_text
            };

            let output = format!(
                "HTTP {} {}\nHeaders:\n{}\n\nBody:\n{}",
                status.as_u16(),
                status.canonical_reason().unwrap_or(""),
                headers_map.join("\n"),
                body_display
            );

            if status.is_success() {
                ToolResult::ok(output)
            } else {
                ToolResult { success: false, output }
            }
        }
        Err(e) => ToolResult::err(format!("HTTP request failed: {e}")),
    }
}

/// Download a file from URL to disk.
pub fn http_download(url: &str, output_path: &str) -> ToolResult {
    if url.is_empty() || output_path.is_empty() {
        return ToolResult::err("URL and output_path are required");
    }

    let expanded = super::shellexpand(output_path);
    let path = std::path::Path::new(&expanded);

    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return ToolResult::err(format!("Failed to create directories: {e}"));
        }
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => return ToolResult::err(format!("Failed to create HTTP client: {e}")),
    };

    match client.get(url).send() {
        Ok(mut response) => {
            if !response.status().is_success() {
                return ToolResult::err(format!("Download failed: HTTP {}", response.status()));
            }

            match std::fs::File::create(path) {
                Ok(mut file) => match copy(&mut response, &mut file) {
                    Ok(bytes_written) => {
                        ToolResult::ok(format!("Downloaded {bytes_written} bytes to {output_path}"))
                    }
                    Err(e) => ToolResult::err(format!("Failed to write file: {e}")),
                },
                Err(e) => ToolResult::err(format!("Failed to create file: {e}")),
            }
        }
        Err(e) => ToolResult::err(format!("Download failed: {e}")),
    }
}

