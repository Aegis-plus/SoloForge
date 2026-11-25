//! Headless table for development:
//!   mcp-dev                  → start the MCP server on a sample world, print the
//!                              Claude Code config, serve until Ctrl-C
//!   mcp-dev --turn "I look around"
//!                            → also run one real GM turn through `claude -p`
//!                              and print the chat afterwards
//! Worlds go to $SOLOFORGE_HOME (default: a temp folder).

use std::sync::Arc;

use soloforge_core::engine::{EventSink, NewWorld};
use soloforge_core::rules::chargen::HeroSpec;
use soloforge_core::world::{Abilities, ChatKind, Store};
use soloforge_core::{gm, mcp, Engine, Settings};

struct Print;
impl EventSink for Print {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        match event {
            "gm://delta" => {
                use std::io::Write;
                print!("{}", payload["text"].as_str().unwrap_or(""));
                let _ = std::io::stdout().flush();
            }
            "gm://status" | "gm://tool" => eprintln!("[{event}] {payload}"),
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let turn = args.iter().position(|a| a == "--turn").and_then(|i| args.get(i + 1)).cloned();
    let root = std::env::var("SOLOFORGE_HOME").map(Into::into).unwrap_or_else(|_| std::env::temp_dir().join("soloforge-dev"));
    let engine = Engine::new(Store::new(root), Settings::load(), Arc::new(Print));
    engine
        .create_world(NewWorld {
            name: "The Ashen Vale".into(),
            pitch: "A valley that burned 40 years ago and never grew back. Quiet dread, a bell that rings at night.".into(),
            tone: vec!["Gritty".into(), "Mystery".into()],
            rules_dial: "balanced".into(),
            hero: HeroSpec { name: "Aria Thorne".into(), ancestry: "Wood Elf".into(), class: "Ranger".into(), scores: Abilities { str: 10, dex: 15, con: 14, int: 10, wis: 13, cha: 8 }, appearance: "Wood elf in a moss-green hooded cloak".into(), background: "Hunts whoever started the fire.".into() },
            companion: false,
        })
        .map_err(anyhow::Error::msg)?;
    let handle = mcp::start(engine.clone()).await?;
    *engine.mcp_config.lock().unwrap() = Some(handle.config_json());
    println!("{}", serde_json::to_string_pretty(&handle.config_json())?);
    if let Some(t) = turn {
        gm::send(engine.clone(), t).map_err(anyhow::Error::msg)?;
        while gm::is_busy(&engine) {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        println!("\n\n--- chat ---");
        for c in engine.world().unwrap().chat {
            match c.kind {
                ChatKind::Gm { text } => println!("GM: {text}"),
                ChatKind::Player { text } => println!("YOU: {text}"),
                ChatKind::Tool { text } | ChatKind::System { text } => println!("  · {text}"),
                ChatKind::Roll { card } => println!("  🎲 {} = {}", card.title, card.lines.iter().map(|l| l.total.to_string()).collect::<Vec<_>>().join(" / ")),
                other => println!("  {other:?}"),
            }
        }
        let w = engine.world().unwrap();
        println!("--- scene: {:?} · tokens {} · journal {} · session {:?}", w.scene().map(|s| &s.name), w.scene().map(|s| s.tokens.len()).unwrap_or(0), w.journal.len(), w.gm_session_id);
        return Ok(());
    }
    tokio::signal::ctrl_c().await?;
    Ok(())
}
