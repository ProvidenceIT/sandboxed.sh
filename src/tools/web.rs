//! Web access tools: search and fetch URLs.

use std::path::Path;

use async_trait::async_trait;
use serde_json::{json, Value};

use super::Tool;

/// Search the web (placeholder - uses DuckDuckGo HTML).
pub struct WebSearch;

#[async_trait]
impl Tool for WebSearch {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information. Returns search results with titles and snippets. Use for finding documentation, examples, or current information."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 5)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value, _workspace: &Path) -> anyhow::Result<String> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' argument"))?;
        let _num_results = args["num_results"].as_u64().unwrap_or(5);

        // Use DuckDuckGo HTML search (no API key needed)
        let encoded_query = urlencoding::encode(query);
        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; OpenAgent/1.0)")
            .build()?;

        let response = client.get(&url).send().await?;
        let html = response.text().await?;

        // Parse results (simple extraction)
        let results = extract_ddg_results(&html);

        if results.is_empty() {
            Ok(format!("No results found for: {}", query))
        } else {
            Ok(results.join("\n\n"))
        }
    }
}

/// Extract search results from DuckDuckGo HTML.
fn extract_ddg_results(html: &str) -> Vec<String> {
    let mut results = Vec::new();

    // Simple regex-free extraction
    // Look for result divs
    for (i, chunk) in html.split("class=\"result__body\"").enumerate().skip(1) {
        if i > 5 {
            break;
        }

        // Extract title
        let title = chunk
            .split("class=\"result__a\"")
            .nth(1)
            .and_then(|s| s.split('>').nth(1))
            .and_then(|s| s.split('<').next())
            .unwrap_or("No title");

        // Extract snippet
        let snippet = chunk
            .split("class=\"result__snippet\"")
            .nth(1)
            .and_then(|s| s.split('>').nth(1))
            .and_then(|s| s.split('<').next())
            .unwrap_or("No snippet");

        // Extract URL
        let url = chunk
            .split("class=\"result__url\"")
            .nth(1)
            .and_then(|s| s.split('>').nth(1))
            .and_then(|s| s.split('<').next())
            .map(|s| s.trim())
            .unwrap_or("");

        if !title.is_empty() && title != "No title" {
            results.push(format!(
                "**{}**\n{}\nURL: {}",
                html_decode(title),
                html_decode(snippet),
                url
            ));
        }
    }

    results
}

/// Basic HTML entity decoding.
fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

/// Fetch content from a URL.
pub struct FetchUrl;

#[async_trait]
impl Tool for FetchUrl {
    fn name(&self) -> &str {
        "fetch_url"
    }

    fn description(&self) -> &str {
        "Fetch the content of a URL. Returns the text content of the page. Useful for reading documentation, APIs, or downloading data."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, args: Value, _workspace: &Path) -> anyhow::Result<String> {
        let url = args["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' argument"))?;

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; OpenAgent/1.0)")
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let response = client.get(url).send().await?;
        let status = response.status();

        if !status.is_success() {
            return Err(anyhow::anyhow!("HTTP error: {}", status));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_default();

        let body = response.text().await?;

        // If HTML, try to extract text content
        let result = if content_type.contains("text/html") {
            extract_text_from_html(&body)
        } else {
            body
        };

        // Truncate if too long
        if result.len() > 20000 {
            Ok(format!(
                "{}... [content truncated, showing first 20000 chars]",
                &result[..20000]
            ))
        } else {
            Ok(result)
        }
    }
}

/// Extract readable text from HTML (simple approach).
fn extract_text_from_html(html: &str) -> String {
    // Remove script and style tags
    let mut text = html.to_string();

    // Remove scripts
    while let Some(start) = text.find("<script") {
        if let Some(end) = text[start..].find("</script>") {
            text = format!("{}{}", &text[..start], &text[start + end + 9..]);
        } else {
            break;
        }
    }

    // Remove styles
    while let Some(start) = text.find("<style") {
        if let Some(end) = text[start..].find("</style>") {
            text = format!("{}{}", &text[..start], &text[start + end + 8..]);
        } else {
            break;
        }
    }

    // Remove all tags
    let mut result = String::new();
    let mut in_tag = false;

    for c in text.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
            result.push(' ');
        } else if !in_tag {
            result.push(c);
        }
    }

    // Clean up whitespace
    let result: String = result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    html_decode(&result)
}

