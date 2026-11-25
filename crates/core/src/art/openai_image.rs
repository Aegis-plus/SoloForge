//! The HTTP client for OpenAI-compatible image endpoints.

use base64::Engine as _;
use serde_json::{json, Value};

use crate::settings::{ApiStyle, ImageSettings};

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().timeout(std::time::Duration::from_secs(180)).build().unwrap_or_default()
}

fn url(base: &str, path: &str) -> String {
    format!("{}/{}", base.trim_end_matches('/'), path)
}

pub fn sniff_ext(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "png"
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        "jpg"
    } else if bytes.len() > 12 && &bytes[8..12] == b"WEBP" {
        "webp"
    } else {
        "png"
    }
}

fn data_url(bytes: &[u8]) -> String {
    let mime = match sniff_ext(bytes) { "jpg" => "image/jpeg", "webp" => "image/webp", _ => "image/png" };
    format!("data:{mime};base64,{}", b64().encode(bytes))
}

fn size_for(aspect: (u32, u32)) -> &'static str {
    match aspect {
        (w, h) if w > h => "1536x1024",
        (w, h) if h > w => "1024x1536",
        _ => "1024x1024",
    }
}

/// Turn an HTTP error into a message that says how to fix it.
fn explain(status: reqwest::StatusCode, body: &str) -> String {
    let detail: String = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/message").and_then(|m| m.as_str()).map(String::from))
        .unwrap_or_else(|| body.chars().take(200).collect());
    let fix = match status.as_u16() {
        401 | 403 => "Paste a fresh API key in Settings › Image Endpoint, then try again.",
        404 => "Check the base URL and model name in Settings › Image Endpoint.",
        402 | 429 => "Your provider says you’re out of credit or rate-limited. Wait a moment or top up, then try again.",
        400 => "The model may not support image output with this API style. Try switching API Style in Settings › Image Endpoint.",
        _ => "Try again in a moment.",
    };
    format!("The image endpoint returned {} {}. {fix} ({detail})", status.as_u16(), status.canonical_reason().unwrap_or(""))
}

async fn fetch_bytes(src: &str) -> Result<Vec<u8>, String> {
    if let Some(rest) = src.strip_prefix("data:") {
        let b = rest.split_once(',').map(|x| x.1).ok_or("malformed data URL")?;
        return b64().decode(b.trim()).map_err(|e| format!("bad base64 image: {e}"));
    }
    if src.starts_with("http") {
        let r = client().get(src).send().await.map_err(|e| format!("couldn’t download the image: {e}"))?;
        return r.bytes().await.map(|b| b.to_vec()).map_err(|e| e.to_string());
    }
    b64().decode(src.trim()).map_err(|e| format!("bad base64 image: {e}"))
}

/// Find an image in a chat-completions response (OpenRouter puts it in
/// `message.images[]`; others use content parts).
pub fn extract_chat_image(v: &Value) -> Option<String> {
    let msg = v.pointer("/choices/0/message")?;
    if let Some(imgs) = msg.get("images").and_then(|i| i.as_array()) {
        for im in imgs {
            if let Some(u) = im.pointer("/image_url/url").and_then(|u| u.as_str()) {
                return Some(u.to_string());
            }
        }
    }
    if let Some(parts) = msg.get("content").and_then(|c| c.as_array()) {
        for p in parts {
            if let Some(u) = p.pointer("/image_url/url").and_then(|u| u.as_str()) {
                return Some(u.to_string());
            }
            if let Some(d) = p.pointer("/inline_data/data").or_else(|| p.pointer("/inlineData/data")).and_then(|u| u.as_str()) {
                return Some(d.to_string());
            }
            if p.get("type").and_then(|t| t.as_str()) == Some("output_image") {
                if let Some(d) = p.get("b64_json").or_else(|| p.get("image_base64")).and_then(|u| u.as_str()) {
                    return Some(d.to_string());
                }
            }
        }
    }
    None
}

pub fn extract_images_api(v: &Value) -> Option<String> {
    let d = v.pointer("/data/0")?;
    d.get("b64_json").or_else(|| d.get("url")).and_then(|x| x.as_str()).map(String::from)
}

pub async fn generate(s: &ImageSettings, prompt: &str, aspect: (u32, u32), refs: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    let key = s.effective_key();
    if key.is_empty() {
        return Err("No API key is set. Add one in Settings › Image Endpoint.".into());
    }
    let c = client();
    match s.api_style {
        ApiStyle::Chat => {
            let mut content = vec![json!({"type": "text", "text": prompt})];
            for r in refs {
                content.push(json!({"type": "image_url", "image_url": {"url": data_url(r)}}));
            }
            let body = json!({
                "model": s.model,
                "messages": [{"role": "user", "content": content}],
                "modalities": ["image", "text"],
                "image_config": {"aspect_ratio": format!("{}:{}", aspect.0, aspect.1)}
            });
            let resp = c.post(url(&s.base_url, "chat/completions")).bearer_auth(&key).json(&body).send().await.map_err(|e| format!("Couldn’t reach the image endpoint: {e}. Check the base URL and your connection."))?;
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(explain(status, &text));
            }
            let v: Value = serde_json::from_str(&text).map_err(|e| format!("The endpoint answered with something other than JSON: {e}"))?;
            let src = extract_chat_image(&v).ok_or("The model answered without an image. Check that the model supports image output (e.g. google/gemini-2.5-flash-image).")?;
            fetch_bytes(&src).await
        }
        ApiStyle::Images => {
            let resp = if refs.is_empty() {
                let body = json!({"model": s.model, "prompt": prompt, "n": 1, "size": size_for(aspect), "response_format": "b64_json"});
                c.post(url(&s.base_url, "images/generations")).bearer_auth(&key).json(&body).send().await
            } else {
                let mut form = reqwest::multipart::Form::new().text("model", s.model.clone()).text("prompt", prompt.to_string()).text("size", size_for(aspect)).text("response_format", "b64_json");
                for (i, r) in refs.iter().enumerate() {
                    let part = reqwest::multipart::Part::bytes(r.clone()).file_name(format!("ref{i}.{}", sniff_ext(r))).mime_str("image/png").map_err(|e| e.to_string())?;
                    form = form.part("image[]", part);
                }
                c.post(url(&s.base_url, "images/edits")).bearer_auth(&key).multipart(form).send().await
            }
            .map_err(|e| format!("Couldn’t reach the image endpoint: {e}. Check the base URL and your connection."))?;
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(explain(status, &text));
            }
            let v: Value = serde_json::from_str(&text).map_err(|e| format!("The endpoint answered with something other than JSON: {e}"))?;
            let src = extract_images_api(&v).ok_or("The endpoint answered without an image.")?;
            fetch_bytes(&src).await
        }
    }
}

/// Settings › Test Connection: generate a tiny image and time it.
pub async fn test_connection(s: &ImageSettings) -> Result<(usize, u128), String> {
    let t = std::time::Instant::now();
    let bytes = generate(s, "A single brass d20 on a dark slate table, studio light.", (1, 1), &[]).await?;
    Ok((bytes.len(), t.elapsed().as_millis()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::post;
    use axum::{Json, Router};

    const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    async fn mock() -> String {
        let png_b64 = b64().encode(PNG);
        let p2 = png_b64.clone();
        let app = Router::new()
            .route(
                "/v1/chat/completions",
                post(move |headers: axum::http::HeaderMap, Json(body): Json<Value>| {
                    let png = png_b64.clone();
                    async move {
                        if headers.get("authorization").and_then(|h| h.to_str().ok()) != Some("Bearer good") {
                            return (axum::http::StatusCode::UNAUTHORIZED, Json(json!({"error": {"message": "bad key"}})));
                        }
                        assert_eq!(body["modalities"], json!(["image", "text"]));
                        (axum::http::StatusCode::OK, Json(json!({"choices": [{"message": {"role": "assistant", "content": "", "images": [{"type": "image_url", "image_url": {"url": format!("data:image/png;base64,{png}")}}]}}]})))
                    }
                }),
            )
            .route(
                "/v1/images/generations",
                post(move |Json(body): Json<Value>| {
                    let png = p2.clone();
                    async move {
                        assert_eq!(body["response_format"], "b64_json");
                        Json(json!({"data": [{"b64_json": png}]}))
                    }
                }),
            );
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
        format!("http://{addr}/v1")
    }

    fn settings(base: &str, style: ApiStyle, key: &str) -> ImageSettings {
        ImageSettings { base_url: base.into(), api_key: key.into(), model: "google/gemini-2.5-flash-image".into(), api_style: style, ..Default::default() }
    }

    #[tokio::test]
    async fn chat_style_returns_png_bytes() {
        let base = mock().await;
        let bytes = generate(&settings(&base, ApiStyle::Chat, "good"), "a map", (3, 2), &[]).await.unwrap();
        assert_eq!(bytes, PNG);
    }

    #[tokio::test]
    async fn images_style_returns_png_bytes() {
        let base = mock().await;
        let bytes = generate(&settings(&base, ApiStyle::Images, "good"), "a map", (1, 1), &[]).await.unwrap();
        assert_eq!(sniff_ext(&bytes), "png");
    }

    #[tokio::test]
    async fn bad_key_explains_the_fix() {
        let base = mock().await;
        let err = generate(&settings(&base, ApiStyle::Chat, "nope"), "x", (1, 1), &[]).await.unwrap_err();
        assert!(err.contains("401") && err.contains("Paste a fresh API key"), "{err}");
    }

    #[test]
    fn extracts_from_content_parts_too() {
        let v = json!({"choices": [{"message": {"content": [{"type": "image_url", "image_url": {"url": "data:image/png;base64,AAAA"}}]}}]});
        assert_eq!(extract_chat_image(&v).unwrap(), "data:image/png;base64,AAAA");
    }
}
