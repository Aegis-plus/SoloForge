//! The Tauri shell: wires the SoloForge engine to the window through
//! commands (UI → Rust) and events (Rust → UI). All game logic lives in
//! `soloforge-core`.

use std::sync::Arc;

use serde::Serialize;
use serde_json::{json, Value};
use soloforge_core::art::{self, openai_image, ImageKind, ImageRequest};
use soloforge_core::engine::{EResult, NewWorld};
use soloforge_core::rules::{chargen, compendium};
use soloforge_core::world::{RollCard, Store, World, WorldSummary};
use soloforge_core::{gm, mcp, Engine, EventSink, Settings};
use tauri::{Emitter, Manager, State};

struct TauriSink(tauri::AppHandle);
impl EventSink for TauriSink {
    fn emit(&self, event: &str, payload: Value) {
        let _ = self.0.emit(event, payload);
    }
}

type Eng<'a> = State<'a, Arc<Engine>>;

#[derive(Serialize)]
struct AppInfo {
    version: &'static str,
    images_root: String,
    mcp_port: u16,
}

#[tauri::command]
fn app_info(e: Eng<'_>) -> AppInfo {
    AppInfo { version: env!("CARGO_PKG_VERSION"), images_root: e.store.worlds_dir().to_string_lossy().into(), mcp_port: e.mcp_port.load(std::sync::atomic::Ordering::SeqCst) }
}

#[tauri::command]
fn list_worlds(e: Eng<'_>) -> Vec<WorldSummary> {
    e.list_worlds()
}
#[tauri::command]
fn create_world(e: Eng<'_>, spec: NewWorld) -> EResult<World> {
    e.create_world(spec)
}
#[tauri::command]
fn open_world(e: Eng<'_>, id: String) -> EResult<World> {
    e.open_world(&id)
}
#[tauri::command]
fn close_world(e: Eng<'_>) {
    e.close_world()
}
#[tauri::command]
fn get_world(e: Eng<'_>) -> Option<World> {
    e.world()
}
#[tauri::command]
fn delete_world(e: Eng<'_>, id: String) -> EResult<()> {
    e.delete_world(&id)
}
#[tauri::command]
fn restore_world(e: Eng<'_>, id: String) -> EResult<()> {
    e.restore_world(&id)
}

#[tauri::command]
async fn send_message(e: Eng<'_>, text: String) -> EResult<String> {
    gm::send(e.inner().clone(), text).map(String::from)
}
#[tauri::command]
fn stop_gm(e: Eng<'_>) -> bool {
    gm::stop(&e)
}
#[tauri::command]
fn gm_busy(e: Eng<'_>) -> bool {
    gm::is_busy(&e)
}

#[tauri::command]
fn roll(e: Eng<'_>, expr: String, label: String) -> EResult<RollCard> {
    e.player_roll(&expr, &label)
}
#[tauri::command]
fn check(e: Eng<'_>, kind: String, adv: String) -> EResult<RollCard> {
    e.player_check(&kind, &adv)
}
#[tauri::command]
fn attack(e: Eng<'_>, attack: String, target: String, adv: String) -> EResult<RollCard> {
    e.player_attack(&attack, &target, &adv)
}
#[tauri::command]
fn move_token(e: Eng<'_>, token_id: String, x: i32, y: i32) -> EResult<()> {
    e.move_token(&token_id, x, y)
}
#[tauri::command]
fn end_turn(e: Eng<'_>) -> EResult<()> {
    e.end_turn()
}
#[tauri::command]
fn ask_oracle(e: Eng<'_>, question: String, likelihood: String) -> EResult<()> {
    e.ask_oracle(&question, &likelihood)
}
#[tauri::command]
fn resolve_proposal(e: Eng<'_>, id: String, accept: bool) -> EResult<()> {
    e.resolve_proposal(&id, accept)
}
#[tauri::command]
fn rest(e: Eng<'_>, long: bool) -> EResult<String> {
    e.rest(long)
}
#[tauri::command]
fn set_story_mode(e: Eng<'_>, on: bool) -> EResult<()> {
    e.set_story_mode(on)
}
#[tauri::command]
fn add_note(e: Eng<'_>, title: String, body: String) -> EResult<()> {
    e.add_note(&title, &body)
}
#[tauri::command]
fn update_notes(e: Eng<'_>, notes: String) -> EResult<()> {
    e.update_notes(&notes)
}

#[tauri::command]
fn get_settings(e: Eng<'_>) -> Settings {
    e.settings().redacted()
}
#[tauri::command]
fn save_settings(e: Eng<'_>, settings: Settings) -> EResult<Settings> {
    e.save_settings(settings)
}
#[tauri::command]
async fn test_image(e: Eng<'_>, settings: Option<Settings>) -> EResult<Value> {
    let mut s = settings.unwrap_or_else(|| e.settings()).image;
    if s.api_key.starts_with('•') {
        s.api_key = e.settings().image.api_key;
    }
    let (bytes, ms) = openai_image::test_connection(&s).await?;
    Ok(json!({"bytes": bytes, "ms": ms}))
}
#[tauri::command]
async fn check_claude(e: Eng<'_>) -> EResult<String> {
    let cmd = e.settings().gm.claude_command;
    let out = tokio::process::Command::new(if cmd.trim().is_empty() { "claude" } else { cmd.trim() })
        .arg("--version")
        .output()
        .await
        .map_err(|_| format!("Can’t find the `{cmd}` command. Install Claude Code, or set its full path here."))?;
    if !out.status.success() {
        return Err("Claude Code is installed but didn’t answer. Run `claude` once in a terminal to finish setup.".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
#[tauri::command]
async fn generate_image(e: Eng<'_>, kind: String, prompt: String, actor_id: Option<String>) -> EResult<()> {
    let kind = match kind.as_str() {
        "map" => ImageKind::Map,
        "token" => ImageKind::Token,
        _ => ImageKind::Art,
    };
    art::spawn(e.inner().clone(), ImageRequest { kind, prompt, actor_id })
}
#[tauri::command]
fn get_compendium() -> compendium::Compendium {
    compendium::all()
}
#[tauri::command]
fn class_list() -> Value {
    json!(chargen::CLASSES)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let store = Store::new(Store::default_root());
            let _ = std::fs::create_dir_all(store.worlds_dir());
            // Let the webview load generated maps and portraits from the worlds folder only.
            let _ = app.asset_protocol_scope().allow_directory(store.worlds_dir(), true);
            let engine = Engine::new(store, Settings::load(), Arc::new(TauriSink(app.handle().clone())));
            let e2 = engine.clone();
            let handle = tauri::async_runtime::block_on(async move { mcp::start(e2).await })?;
            *engine.mcp_config.lock().unwrap() = Some(handle.config_json());
            app.manage(engine);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info, list_worlds, create_world, open_world, close_world, get_world, delete_world, restore_world,
            send_message, stop_gm, gm_busy, roll, check, attack, move_token, end_turn, ask_oracle, resolve_proposal,
            rest, set_story_mode, add_note, update_notes, get_settings, save_settings, test_image, check_claude,
            generate_image, get_compendium, class_list
        ])
        .run(tauri::generate_context!())
        .expect("error while running SoloForge");
}
