//! The Game Master: Claude Code, run headless (`claude -p … --output-format
//! stream-json`) with only the SoloForge table tools, resuming the world's
//! session so it remembers the campaign.

pub mod stream;

use std::process::Stdio;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde_json::json;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::engine::Engine;
use crate::world::ChatKind;
use stream::{parse_line, short_tool, GmEvent};

pub const GM_PROMPT: &str = include_str!("../../../../assets/gm_prompt.md");

/// The message Claude gets this turn: what the player said plus the table
/// events since its last turn.
pub fn build_turn_prompt(player_text: &str, events: &[String], first_turn: bool, pitch: &str, style: &str) -> String {
    let mut p = String::new();
    if first_turn {
        p.push_str("This is session zero of a new world. Read get_state, then set the opening scene: create a scene (generate_map if a battle map fits), place the hero, write the first quest in the journal, and narrate the opening.\n");
        if !pitch.is_empty() {
            p.push_str(&format!("World pitch from the player: {pitch}\n"));
        }
    }
    if !events.is_empty() {
        p.push_str("Table events since your last turn:\n");
        for e in events {
            p.push_str(&format!("- {e}\n"));
        }
    }
    p.push_str(&format!("Narration style: {style}.\n"));
    if player_text.trim().is_empty() {
        p.push_str("The player waits for you to continue.");
    } else {
        p.push_str(&format!("The player says:\n{}", player_text.trim()));
    }
    p
}

pub fn claude_args(prompt: &str, mcp_config: &str, resume: Option<&str>, model: &str) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "-p".into(),
        prompt.into(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
        "--include-partial-messages".into(),
        "--mcp-config".into(),
        mcp_config.into(),
        "--strict-mcp-config".into(),
        "--tools".into(),
        "".into(),
        "--allowedTools".into(),
        "mcp__vtt__*".into(),
        "--append-system-prompt".into(),
        GM_PROMPT.into(),
    ];
    if let Some(r) = resume {
        a.push("--resume".into());
        a.push(r.into());
    }
    if !model.trim().is_empty() {
        a.push("--model".into());
        a.push(model.trim().into());
    }
    a
}

fn status(engine: &Engine, state: &str, detail: &str) {
    engine.emit("gm://status", json!({"state": state, "detail": detail}));
}

pub fn is_busy(engine: &Engine) -> bool {
    engine.gm_busy.load(Ordering::SeqCst)
}

/// Stop the running GM turn. Changes it already made to the table stay.
pub fn stop(engine: &Engine) -> bool {
    if let Some(tx) = engine.gm_cancel.lock().unwrap().take() {
        let _ = tx.send(());
        true
    } else {
        false
    }
}

/// Send the player's message to the GM. If a turn is running, it's queued.
pub fn send(engine: Arc<Engine>, text: String) -> Result<&'static str, String> {
    if !text.trim().is_empty() {
        engine.with_world(|w| {
            w.push_chat(ChatKind::Player { text: text.trim().to_string() });
            w.story_choices.clear();
            Ok(())
        })?;
    }
    if engine.gm_busy.swap(true, Ordering::SeqCst) {
        engine.gm_queue.lock().unwrap().push(text);
        return Ok("queued");
    }
    let h = tokio::runtime::Handle::try_current().map_err(|_| "no async runtime".to_string())?;
    h.spawn(run_loop(engine, text));
    Ok("started")
}

async fn run_loop(engine: Arc<Engine>, mut text: String) {
    loop {
        if let Err(e) = run_turn(&engine, &text).await {
            let _ = engine.with_world(|w| {
                w.push_chat(ChatKind::System { text: e.clone() });
                Ok(())
            });
            status(&engine, "error", &e);
        }
        let next: Vec<String> = std::mem::take(&mut *engine.gm_queue.lock().unwrap());
        if next.is_empty() {
            engine.gm_busy.store(false, Ordering::SeqCst);
            status(&engine, "idle", "");
            return;
        }
        text = next.join("\n");
    }
}

async fn run_turn(engine: &Arc<Engine>, player_text: &str) -> Result<(), String> {
    let world = engine.world().ok_or("No world is open.")?;
    let settings = engine.settings().gm;
    let cfg = engine.mcp_config.lock().unwrap().clone().ok_or("The table server isn’t running. Restart SoloForge.")?;
    let dir = engine.store.world_dir(&world.id);
    let cfg_path = dir.join(".mcp.json");
    std::fs::write(&cfg_path, serde_json::to_vec_pretty(&cfg).unwrap_or_default()).map_err(|e| format!("Couldn’t write the GM config: {e}"))?;

    let events = engine.with_world(|w| Ok(std::mem::take(&mut w.pending_events)))?;
    let first = world.gm_session_id.is_none() && !world.chat.iter().any(|c| matches!(c.kind, ChatKind::Gm { .. }));
    let prompt = build_turn_prompt(player_text, &events, first, &world.pitch, &settings.style);

    let mut resume = world.gm_session_id.clone();
    for attempt in 0..2 {
        status(engine, "thinking", "");
        let args = claude_args(&prompt, &cfg_path.to_string_lossy(), resume.as_deref(), &settings.model);
        let outcome = run_claude(engine, &settings.claude_command, &args, &dir).await?;
        match outcome {
            TurnOutcome::Done { session_id } => {
                if let Some(sid) = session_id {
                    engine.with_world(|w| {
                        w.gm_session_id = Some(sid);
                        Ok(())
                    })?;
                }
                return Ok(());
            }
            TurnOutcome::Stopped => {
                engine.with_world(|w| {
                    w.push_chat(ChatKind::System { text: "You stopped the GM mid-turn. Changes it already made to the table stay.".into() });
                    Ok(())
                })?;
                return Ok(());
            }
            TurnOutcome::Failed(msg) => {
                // A lost session: start a fresh one once.
                if attempt == 0 && resume.is_some() && msg.to_lowercase().contains("conversation") {
                    resume = None;
                    engine.with_world(|w| {
                        w.gm_session_id = None;
                        Ok(())
                    })?;
                    continue;
                }
                return Err(msg);
            }
        }
    }
    Ok(())
}

enum TurnOutcome {
    Done { session_id: Option<String> },
    Stopped,
    Failed(String),
}

async fn run_claude(engine: &Arc<Engine>, cmd: &str, args: &[String], cwd: &std::path::Path) -> Result<TurnOutcome, String> {
    let mut child = tokio::process::Command::new(if cmd.trim().is_empty() { "claude" } else { cmd.trim() })
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("Can’t find the `{cmd}` command. Install Claude Code, or set its full path in Settings › Game Master.")
            } else {
                format!("Couldn’t start Claude Code: {e}")
            }
        })?;
    let (tx, mut rx) = tokio::sync::oneshot::channel();
    *engine.gm_cancel.lock().unwrap() = Some(tx);
    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let mut session_id = None;
    let mut streaming = String::new();
    let mut result: Option<(bool, String)> = None;
    loop {
        tokio::select! {
            _ = &mut rx => {
                let _ = child.kill().await;
                flush(engine, &mut streaming);
                return Ok(TurnOutcome::Stopped);
            }
            line = lines.next_line() => {
                let Ok(Some(line)) = line else { break };
                match parse_line(&line) {
                    Some(GmEvent::Init { session_id: s }) => session_id = Some(s),
                    Some(GmEvent::MessageStart) => streaming.clear(),
                    Some(GmEvent::TextDelta(t)) => {
                        streaming.push_str(&t);
                        engine.emit("gm://delta", json!({"text": t}));
                    }
                    Some(GmEvent::ToolStart(n)) => status(engine, "acting", short_tool(&n)),
                    Some(GmEvent::Assistant { text, .. }) => {
                        if !text.trim().is_empty() {
                            streaming.clear();
                            let t = text.trim().to_string();
                            engine.with_world(|w| { w.push_chat(ChatKind::Gm { text: t }); Ok(()) })?;
                            engine.emit("gm://message-done", json!({}));
                        }
                    }
                    Some(GmEvent::Result { is_error, text, session_id: s }) => {
                        if s.is_some() { session_id = s; }
                        result = Some((is_error, text));
                    }
                    _ => {}
                }
            }
        }
    }
    *engine.gm_cancel.lock().unwrap() = None;
    let status_code = child.wait().await.map_err(|e| e.to_string())?;
    flush(engine, &mut streaming);
    let mut err_text = String::new();
    {
        use tokio::io::AsyncReadExt;
        let _ = stderr.read_to_string(&mut err_text).await;
    }
    match result {
        Some((false, _)) => Ok(TurnOutcome::Done { session_id }),
        Some((true, text)) => Ok(TurnOutcome::Failed(explain_failure(&text, &err_text))),
        None if status_code.success() => Ok(TurnOutcome::Done { session_id }),
        None => Ok(TurnOutcome::Failed(explain_failure("", &err_text))),
    }
}

/// Keep any partial narration that never got a full assistant message.
fn flush(engine: &Arc<Engine>, streaming: &mut String) {
    let t = std::mem::take(streaming);
    if !t.trim().is_empty() {
        let _ = engine.with_world(|w| {
            w.push_chat(ChatKind::Gm { text: t.trim().to_string() });
            Ok(())
        });
    }
}

pub fn explain_failure(result: &str, stderr: &str) -> String {
    let all = format!("{result}\n{stderr}").to_lowercase();
    if all.contains("log in") || all.contains("login") || all.contains("not authenticated") || all.contains("invalid api key") || all.contains("/login") {
        "Can’t reach Claude Code: it isn’t signed in. Open a terminal, run `claude` once and sign in, then try again. Your message is kept.".into()
    } else if all.contains("no conversation found") {
        "The GM’s previous conversation is gone; starting a new one.".into()
    } else if all.contains("rate limit") || all.contains("usage limit") {
        "Claude Code hit a usage limit. Wait a bit and try again; your message is kept.".into()
    } else {
        let detail: String = format!("{} {}", result.trim(), stderr.trim()).trim().chars().take(300).collect();
        format!("The GM couldn’t finish this turn. Try again. ({detail})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_includes_events_and_session_zero() {
        let p = build_turn_prompt("I open the door", &["Player moved Aria 10 ft to (3, 4).".into()], true, "burned valley", "vivid");
        assert!(p.contains("session zero") && p.contains("burned valley") && p.contains("moved Aria") && p.ends_with("I open the door"));
    }

    #[test]
    fn args_restrict_tools_and_resume() {
        let a = claude_args("hi", "/w/.mcp.json", Some("abc"), "sonnet");
        let j = a.join(" ");
        assert!(j.contains("--strict-mcp-config") && j.contains("--allowedTools mcp__vtt__*") && j.contains("--resume abc") && j.contains("--model sonnet"));
        let ti = a.iter().position(|x| x == "--tools").unwrap();
        assert_eq!(a[ti + 1], "");
    }

    #[test]
    fn failures_say_how_to_fix() {
        assert!(explain_failure("Invalid API key · Please run /login", "").contains("sign in"));
        assert!(explain_failure("", "boom").contains("Try again"));
    }
}
