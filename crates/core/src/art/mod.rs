//! Image generation through any OpenAI-compatible endpoint (Nano Banana via
//! OpenRouter, LiteLLM, your own gateway). Two API styles: chat completions
//! with image output, and the Images API.

pub mod openai_image;

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::engine::Engine;
use crate::world::{new_id, ChatKind, ImageStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageKind {
    Map,
    Token,
    Art,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequest {
    pub kind: ImageKind,
    pub prompt: String,
    /// For tokens/portraits.
    pub actor_id: Option<String>,
}

/// Prompt template per kind, plus the world's art style.
pub fn build_prompt(kind: ImageKind, prompt: &str, style: &str, grid: Option<(i32, i32)>) -> String {
    let body = match kind {
        ImageKind::Map => format!(
            "Top-down orthographic battle map for a tabletop RPG, seen from directly above, no perspective, no grid lines, no text or labels, no characters. {} {}",
            grid.map(|(w, h)| format!("The map covers {w} by {h} squares of 5 feet.")).unwrap_or_default(),
            prompt
        ),
        ImageKind::Token => format!("Character portrait for a tabletop RPG token: head and shoulders, centered, facing the viewer, plain dark background, no text. {prompt}"),
        ImageKind::Art => format!("Cinematic illustration of a scene from a tabletop RPG adventure, no text. {prompt}"),
    };
    if style.trim().is_empty() { body } else { format!("{body} Style: {}", style.trim()) }
}

pub fn aspect(kind: ImageKind, grid: Option<(i32, i32)>) -> (u32, u32) {
    match (kind, grid) {
        (ImageKind::Token, _) => (1, 1),
        (ImageKind::Map, Some((w, h))) if w > h => (3, 2),
        (ImageKind::Map, Some((w, h))) if h > w => (2, 3),
        (ImageKind::Map, _) => (1, 1),
        (ImageKind::Art, _) => (16, 9),
    }
}

/// Queue an image; the world shows it as pending and updates when it lands.
pub fn spawn(engine: Arc<Engine>, req: ImageRequest) -> Result<(), String> {
    let handle = tokio::runtime::Handle::try_current().map_err(|_| "no async runtime".to_string())?;
    let settings = engine.settings().image;
    if !settings.enabled() {
        return Err("No image endpoint is set up. Add one in Settings › Image Endpoint.".into());
    }
    // Mark pending and gather context.
    let chat_id = new_id();
    let (grid, reference) = engine.with_world(|w| {
        let grid = w.scene().map(|s| (s.width, s.height));
        match req.kind {
            ImageKind::Map => {
                if let Some(s) = w.scene_mut() {
                    s.map_status = ImageStatus::Pending;
                }
            }
            ImageKind::Art => {
                w.chat.push(crate::world::ChatEntry { id: chat_id.clone(), ts: crate::world::now_ms(), kind: ChatKind::Image { file: None, prompt: req.prompt.clone(), status: ImageStatus::Pending, error: String::new() } });
            }
            ImageKind::Token => {}
        }
        let reference = if req.kind == ImageKind::Art && settings.hero_consistency { w.pc().and_then(|p| p.portrait.clone()) } else { None };
        Ok((grid, reference))
    })?;
    let dir = engine.images_dir().ok_or("no world open")?;
    let scene_id = engine.world().and_then(|w| w.active_scene.clone());
    handle.spawn(async move {
        let prompt = build_prompt(req.kind, &req.prompt, &settings.art_style, grid);
        let refs: Vec<Vec<u8>> = reference.and_then(|f| std::fs::read(dir.join(f)).ok()).into_iter().collect();
        let result = openai_image::generate(&settings, &prompt, aspect(req.kind, grid), &refs).await;
        let outcome = match result {
            Ok(bytes) => {
                let ext = openai_image::sniff_ext(&bytes);
                let file = format!("{}-{}.{ext}", match req.kind { ImageKind::Map => "map", ImageKind::Token => "token", ImageKind::Art => "art" }, new_id());
                match tokio::fs::create_dir_all(&dir).await.and(Ok(())).and_then(|_| std::fs::write(dir.join(&file), &bytes)) {
                    Ok(()) => Ok(file),
                    Err(e) => Err(format!("Couldn’t save the image: {e}")),
                }
            }
            Err(e) => Err(e),
        };
        let _ = engine.with_world(|w| {
            match (&outcome, req.kind) {
                (Ok(file), ImageKind::Map) => {
                    if let Some(s) = w.scenes.iter_mut().find(|s| Some(&s.id) == scene_id.as_ref()) {
                        s.background = Some(file.clone());
                        s.map_status = ImageStatus::Ready;
                    }
                }
                (Err(_), ImageKind::Map) => {
                    if let Some(s) = w.scenes.iter_mut().find(|s| Some(&s.id) == scene_id.as_ref()) {
                        s.map_status = ImageStatus::Failed;
                    }
                }
                (Ok(file), ImageKind::Token) => {
                    if let Some(id) = &req.actor_id {
                        if let Some(a) = w.actor_mut(id) {
                            a.portrait = Some(file.clone());
                        }
                    }
                }
                _ => {}
            }
            if req.kind == ImageKind::Art {
                if let Some(c) = w.chat.iter_mut().find(|c| c.id == chat_id) {
                    c.kind = match &outcome {
                        Ok(f) => ChatKind::Image { file: Some(f.clone()), prompt: req.prompt.clone(), status: ImageStatus::Ready, error: String::new() },
                        Err(e) => ChatKind::Image { file: None, prompt: req.prompt.clone(), status: ImageStatus::Failed, error: e.clone() },
                    };
                }
            } else if let Err(e) = &outcome {
                w.push_chat(ChatKind::System { text: format!("Image not generated. {e}") });
            }
            Ok(())
        });
        engine.emit("art://done", json!({"kind": req.kind, "ok": outcome.is_ok()}));
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prompts_and_aspects() {
        let p = build_prompt(ImageKind::Map, "a ruined chapel", "ink", Some((20, 14)));
        assert!(p.contains("Top-down") && p.contains("20 by 14") && p.ends_with("Style: ink"));
        assert_eq!(aspect(ImageKind::Map, Some((20, 14))), (3, 2));
        assert_eq!(aspect(ImageKind::Token, None), (1, 1));
    }
}
