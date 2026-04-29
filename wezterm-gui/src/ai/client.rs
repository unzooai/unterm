//! AI API client — calls Claude, OpenAI, and Gemini APIs.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use super::models::ModelProvider;

/// Make an AI completion request. Returns the response text.
pub fn complete(
    provider: &ModelProvider,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String> {
    if api_key.is_empty() {
        return Err(anyhow!(
            "No API key configured for {:?}. Set ai_{}_api_key in your config.",
            provider,
            match provider {
                ModelProvider::Claude => "claude",
                ModelProvider::OpenAI => "openai",
                ModelProvider::Gemini => "gemini",
                ModelProvider::Custom => "custom",
            }
        ));
    }

    match provider {
        ModelProvider::Claude => complete_claude(api_key, model, system_prompt, user_prompt),
        ModelProvider::OpenAI => complete_openai(api_key, model, system_prompt, user_prompt),
        ModelProvider::Gemini => complete_gemini(api_key, model, system_prompt, user_prompt),
        ModelProvider::Custom => Err(anyhow!("Custom endpoint not yet implemented")),
    }
}

fn http_post(url: &str, headers: &[(&str, &str)], body: &[u8]) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow!("Failed to build HTTP client: {e}"))?;

    let mut req = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(body.to_vec());

    for (key, value) in headers {
        req = req.header(*key, *value);
    }

    let res = req
        .send()
        .map_err(|e| anyhow!("HTTP request failed: {e}"))?;

    let status = res.status();
    let response_body = res
        .bytes()
        .map_err(|e| anyhow!("Failed to read response: {e}"))?
        .to_vec();

    if !status.is_success() {
        let body_text = String::from_utf8_lossy(&response_body);
        return Err(anyhow!(
            "API error (HTTP {}): {}",
            status.as_u16(),
            body_text.chars().take(500).collect::<String>()
        ));
    }

    Ok(response_body)
}

fn complete_claude(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String> {
    let body = json!({
        "model": model,
        "max_tokens": 1024,
        "system": system_prompt,
        "messages": [
            {"role": "user", "content": user_prompt}
        ]
    });

    let body_bytes = serde_json::to_vec(&body)?;
    let response = http_post(
        "https://api.anthropic.com/v1/messages",
        &[("x-api-key", api_key), ("anthropic-version", "2023-06-01")],
        &body_bytes,
    )?;

    let resp: Value = serde_json::from_slice(&response)?;
    resp["content"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| {
            anyhow!(
                "Unexpected Claude response format: {}",
                serde_json::to_string_pretty(&resp)
                    .unwrap_or_default()
                    .chars()
                    .take(300)
                    .collect::<String>()
            )
        })
}

fn complete_openai(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String> {
    let body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt}
        ],
        "max_tokens": 1024,
    });

    let body_bytes = serde_json::to_vec(&body)?;
    let auth = format!("Bearer {api_key}");
    let response = http_post(
        "https://api.openai.com/v1/chat/completions",
        &[("Authorization", &auth)],
        &body_bytes,
    )?;

    let resp: Value = serde_json::from_slice(&response)?;
    resp["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Unexpected OpenAI response format"))
}

fn complete_gemini(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let body = json!({
        "system_instruction": {
            "parts": [{"text": system_prompt}]
        },
        "contents": [{
            "parts": [{"text": user_prompt}]
        }],
        "generationConfig": {
            "maxOutputTokens": 1024,
        }
    });

    let body_bytes = serde_json::to_vec(&body)?;
    let response = http_post(&url, &[], &body_bytes)?;

    let resp: Value = serde_json::from_slice(&response)?;
    resp["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Unexpected Gemini response format"))
}

/// Generate a ghost text command completion suggestion.
pub fn ghost_text_complete(
    provider: &ModelProvider,
    api_key: &str,
    model: &str,
    shell_type: &str,
    cwd: &str,
    screen_context: &str,
    partial_input: &str,
) -> Result<String> {
    let system = format!(
        "You are a terminal command completion engine. The user is working in a {} shell.\n\
         Current directory: {}\n\
         You MUST respond with ONLY the completion text (the rest of the command), nothing else.\n\
         Do NOT include the partial input that's already typed.\n\
         Do NOT include any explanation, quotes, or markdown.\n\
         If you cannot suggest a completion, respond with an empty string.",
        shell_type, cwd
    );

    let user = format!(
        "Recent terminal output:\n```\n{}\n```\n\nPartial command being typed: `{}`\n\nComplete this command:",
        screen_context.chars().take(2000).collect::<String>(),
        partial_input
    );

    complete(provider, api_key, model, &system, &user)
}

/// Analyze a terminal error and suggest a fix.
pub fn analyze_error(
    provider: &ModelProvider,
    api_key: &str,
    model: &str,
    shell_type: &str,
    screen_context: &str,
) -> Result<(String, Option<String>)> {
    let system = format!(
        "You are a terminal error analysis assistant for {} shell.\n\
         Analyze the error in the terminal output and provide:\n\
         1. A brief explanation of what went wrong (1-2 sentences)\n\
         2. A fix command if applicable\n\n\
         Respond in this exact JSON format:\n\
         {{\"explanation\": \"...\", \"fix_command\": \"...\" or null}}",
        shell_type
    );

    let response = complete(provider, api_key, model, &system, screen_context)?;
    let cleaned = strip_code_fences(&response);

    // Parse the JSON response
    if let Ok(parsed) = serde_json::from_str::<Value>(cleaned) {
        let explanation = parsed["explanation"]
            .as_str()
            .unwrap_or("Unable to analyze error")
            .to_string();
        let fix = parsed["fix_command"].as_str().map(|s| s.to_string());
        Ok((explanation, fix))
    } else {
        // Fallback: treat entire response as explanation
        Ok((response.trim().to_string(), None))
    }
}

/// Chat with AI about the terminal context.
pub fn chat(
    provider: &ModelProvider,
    api_key: &str,
    model: &str,
    shell_type: &str,
    screen_context: &str,
    user_message: &str,
) -> Result<String> {
    let system = format!(
        "You are a helpful terminal assistant. The user is working in a {} shell.\n\
         You can see the recent terminal output below. Help the user with their question.\n\
         If you suggest commands, wrap them in ``` code blocks.\n\
         Be concise and practical.",
        shell_type
    );

    let user = format!(
        "Terminal context:\n```\n{}\n```\n\nUser: {}",
        screen_context.chars().take(3000).collect::<String>(),
        user_message
    );

    complete(provider, api_key, model, &system, &user)
}

/// Strip markdown code fences from AI responses (e.g. ```json ... ```)
fn strip_code_fences(s: &str) -> &str {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        // Skip the language tag line (e.g. "json\n")
        let rest = rest
            .trim_start_matches(|c: char| c != '\n')
            .trim_start_matches('\n');
        if let Some(inner) = rest.strip_suffix("```") {
            return inner.trim();
        }
    }
    trimmed
}

/// Suggest what to do next based on the terminal context.
pub fn suggest_next_step(
    provider: &ModelProvider,
    api_key: &str,
    model: &str,
    shell_type: &str,
    screen_context: &str,
) -> Result<(String, Option<String>)> {
    let system = format!(
        "You are a terminal workflow assistant for {} shell.\n\
         Based on the recent terminal output, suggest what the user should do next.\n\
         For example: after 'git commit', suggest 'git push'. After installing a package, suggest running the app.\n\n\
         Respond in this exact JSON format:\n\
         {{\"suggestion\": \"brief suggestion text\", \"command\": \"suggested command\" or null}}",
        shell_type
    );

    let response = complete(provider, api_key, model, &system, screen_context)?;
    let cleaned = strip_code_fences(&response);

    if let Ok(parsed) = serde_json::from_str::<Value>(cleaned) {
        let suggestion = parsed["suggestion"]
            .as_str()
            .unwrap_or("No suggestion")
            .to_string();
        let command = parsed["command"].as_str().map(|s| s.to_string());
        Ok((suggestion, command))
    } else {
        Ok((response.trim().to_string(), None))
    }
}
