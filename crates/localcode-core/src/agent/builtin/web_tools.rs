use async_trait::async_trait;
use crate::agent::tools::{Tool, ToolContext};
use crate::CoreError;

pub struct WebSearchTool;

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str { "web_search" }
    fn description(&self) -> &str { "Search the web for information using DuckDuckGo. Returns top search results with titles, URLs, and snippets." }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value, _ctx: &ToolContext) -> Result<String, CoreError> {
        let query = args["query"].as_str().unwrap_or("");
        if query.is_empty() {
            return Err(CoreError::ToolError("query is required".to_string()));
        }

        let encoded_query = urlencoding::encode(query);
        let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; LocalCode/1.0)")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| CoreError::ToolError(format!("Failed to create HTTP client: {}", e)))?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| CoreError::ToolError(format!("Search request failed: {}", e)))?;

        let body = response
            .text()
            .await
            .map_err(|e| CoreError::ToolError(format!("Failed to read response: {}", e)))?;

        // Parse search results from DuckDuckGo HTML
        let results = parse_ddg_results(&body);

        if results.is_empty() {
            return Ok(format!("No results found for: {}", query));
        }

        let mut output = format!("Search results for: {}\n\n", query);
        for (i, result) in results.iter().enumerate().take(5) {
            output.push_str(&format!(
                "{}. {}\n   URL: {}\n   {}\n\n",
                i + 1,
                result.title,
                result.url,
                result.snippet
            ));
        }

        Ok(output)
    }
}

struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

fn parse_ddg_results(html: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // Simple HTML parsing for DuckDuckGo results
    // Results are in <a class="result__a" href="...">title</a>
    // Snippets in <a class="result__snippet">...</a>
    let mut pos = 0;
    while let Some(link_start) = html[pos..].find("class=\"result__a\"") {
        let abs_pos = pos + link_start;

        // Find href
        let before = &html[abs_pos.saturating_sub(200)..abs_pos];
        let href = if let Some(href_start) = before.rfind("href=\"") {
            let href_content = &before[href_start + 6..];
            if let Some(href_end) = href_content.find('"') {
                let raw_url = &href_content[..href_end];
                // DuckDuckGo wraps URLs, extract actual URL
                if let Some(uddg) = raw_url.find("uddg=") {
                    let encoded = &raw_url[uddg + 5..];
                    let decoded = encoded.split('&').next().unwrap_or(encoded);
                    urlencoding::decode(decoded).unwrap_or_else(|_| decoded.into()).to_string()
                } else {
                    raw_url.to_string()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Find title (content between > and </a>)
        let after_class = &html[abs_pos..];
        let title = if let Some(gt) = after_class.find('>') {
            let content = &after_class[gt + 1..];
            if let Some(end) = content.find("</a>") {
                strip_tags(&content[..end])
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Find snippet
        let snippet_search = &html[abs_pos..];
        let snippet = if let Some(snippet_start) = snippet_search.find("result__snippet") {
            let s = &snippet_search[snippet_start..];
            if let Some(gt) = s.find('>') {
                let content = &s[gt + 1..];
                if let Some(end) = content.find("</") {
                    strip_tags(&content[..end])
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if !title.is_empty() && !href.is_empty() {
            results.push(SearchResult {
                title,
                url: href,
                snippet,
            });
        }

        pos = abs_pos + 20;
        if results.len() >= 5 {
            break;
        }
    }

    results
}

fn strip_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result.trim().to_string()
}
