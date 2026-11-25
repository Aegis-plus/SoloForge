//! App settings in `~/.config/soloforge/config.json` (owner-only permissions).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
#[derive(Default)]
pub struct Settings {
    pub gm: GmSettings,
    pub image: ImageSettings,
    pub appearance: Appearance,
    pub rules: RuleToggles,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GmSettings {
    pub claude_command: String,
    /// Empty = whatever the user's Claude Code defaults to.
    pub model: String,
    /// terse | vivid | novelistic
    pub style: String,
    pub secret_rolls: bool,
    pub ask_before_sheet_changes: bool,
}
impl Default for GmSettings {
    fn default() -> Self {
        GmSettings { claude_command: "claude".into(), model: String::new(), style: "vivid".into(), secret_rolls: true, ask_before_sheet_changes: true }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApiStyle {
    /// `POST {base}/chat/completions` with `modalities: ["image","text"]` (OpenRouter's Nano Banana).
    #[default]
    Chat,
    /// `POST {base}/images/generations` (+ `/images/edits` for references).
    Images,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ImageSettings {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub api_style: ApiStyle,
    pub art_style: String,
    pub hero_consistency: bool,
    pub gm_can_generate: bool,
}
impl Default for ImageSettings {
    fn default() -> Self {
        ImageSettings {
            base_url: "https://openrouter.ai/api/v1".into(),
            api_key: String::new(),
            model: "google/gemini-2.5-flash-image".into(),
            api_style: ApiStyle::Chat,
            art_style: "Painterly dark-fantasy illustration, cool moonlight and warm lantern light, ink outlines.".into(),
            hero_consistency: true,
            gm_can_generate: true,
        }
    }
}
impl ImageSettings {
    /// Key from settings, else `SOLOFORGE_IMAGE_API_KEY`.
    pub fn effective_key(&self) -> String {
        if !self.api_key.trim().is_empty() {
            return self.api_key.trim().to_string();
        }
        std::env::var("SOLOFORGE_IMAGE_API_KEY").unwrap_or_default()
    }
    pub fn enabled(&self) -> bool {
        !self.base_url.trim().is_empty() && !self.model.trim().is_empty() && !self.effective_key().is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Appearance {
    /// dark | light | system
    pub theme: String,
    pub reduce_motion: bool,
    pub high_contrast: bool,
    pub narration_px: u8,
    pub announce: bool,
}
impl Default for Appearance {
    fn default() -> Self {
        Appearance { theme: "dark".into(), reduce_motion: false, high_contrast: false, narration_px: 17, announce: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RuleToggles {
    pub fate: bool,
    pub minions: bool,
    pub oracle_events: bool,
    pub lethal: bool,
}
impl Default for RuleToggles {
    fn default() -> Self {
        RuleToggles { fate: true, minions: true, oracle_events: true, lethal: false }
    }
}

pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("SOLOFORGE_CONFIG") {
        return PathBuf::from(p);
    }
    directories::ProjectDirs::from("dev", "soloforge", "soloforge")
        .map(|d| d.config_dir().join("config.json"))
        .unwrap_or_else(|| PathBuf::from(".soloforge/config.json"))
}

impl Settings {
    pub fn load() -> Settings {
        std::fs::read(config_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }
    pub fn save(&self) -> std::io::Result<()> {
        let p = config_path();
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        let tmp = p.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).unwrap_or_default())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(tmp, p)
    }
    /// Settings as sent to the UI: the key is masked.
    pub fn redacted(&self) -> Settings {
        let mut s = self.clone();
        if !s.image.api_key.is_empty() {
            let k = &s.image.api_key;
            let tail: String = k.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
            s.image.api_key = format!("••••••••{tail}");
        }
        s
    }
}
