//! Drive the MCP table server over real HTTP, the way Claude Code does.

use std::sync::Arc;

use serde_json::{json, Value};
use soloforge_core::engine::NewWorld;
use soloforge_core::rules::chargen::HeroSpec;
use soloforge_core::world::{Abilities, ChatKind, Store};
use soloforge_core::{mcp, Engine, NullSink, Settings};

async fn setup() -> (Arc<Engine>, mcp::McpHandle) {
    let dir = std::env::temp_dir().join(format!("sf-mcp-{}", soloforge_core::world::new_id()));
    let engine = Engine::new(Store::new(dir), Settings::default(), Arc::new(NullSink));
    engine.seed(12);
    engine
        .create_world(NewWorld {
            name: "Test".into(),
            pitch: String::new(),
            tone: vec![],
            rules_dial: String::new(),
            hero: HeroSpec { name: "Aria Thorne".into(), ancestry: "Wood Elf".into(), class: "Ranger".into(), scores: Abilities { str: 10, dex: 15, con: 14, int: 10, wis: 13, cha: 8 }, appearance: String::new(), background: String::new() },
            companion: false,
        })
        .unwrap();
    let h = mcp::start(engine.clone()).await.unwrap();
    (engine, h)
}

async fn rpc(h: &mcp::McpHandle, method: &str, params: Value) -> Value {
    let r = reqwest::Client::new()
        .post(h.url())
        .bearer_auth(&h.token)
        .json(&json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}))
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success(), "{}", r.status());
    r.json().await.unwrap()
}

async fn call(h: &mcp::McpHandle, name: &str, args: Value) -> (bool, String) {
    let v = rpc(h, "tools/call", json!({"name": name, "arguments": args})).await;
    let text = v["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string();
    (v["result"]["isError"].as_bool().unwrap_or(true), text)
}

#[tokio::test]
async fn handshake_and_tool_list() {
    let (_e, h) = setup().await;
    let init = rpc(&h, "initialize", json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}})).await;
    assert_eq!(init["result"]["serverInfo"]["name"], "soloforge-table");
    let list = rpc(&h, "tools/list", json!({})).await;
    let names: Vec<&str> = list["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    for n in ["get_state", "roll", "attack", "move_token", "start_combat", "generate_image", "journal_write"] {
        assert!(names.contains(&n), "{n} missing");
    }
    // notifications get 202 and no body
    let r = reqwest::Client::new().post(h.url()).bearer_auth(&h.token).json(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).send().await.unwrap();
    assert_eq!(r.status(), 202);
}

#[tokio::test]
async fn rejects_requests_without_the_token() {
    let (_e, h) = setup().await;
    let r = reqwest::Client::new().post(h.url()).json(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})).send().await.unwrap();
    assert_eq!(r.status(), 401);
}

#[tokio::test]
async fn an_encounter_plays_out_through_tools() {
    let (e, h) = setup().await;
    assert!(!call(&h, "create_scene", json!({"name": "Ruined Chapel", "description": "rain, a brazier", "width": 22, "height": 15})).await.0);
    assert!(!call(&h, "create_actor", json!({"name": "Goblin A", "compendium": "goblin"})).await.0);
    assert!(!call(&h, "create_actor", json!({"name": "Goblin B", "compendium": "goblin"})).await.0);
    let (err, msg) = call(&h, "create_actor", json!({"name": "Goblin B", "compendium": "goblin"})).await;
    assert!(err && msg.contains("already exists"));
    assert!(!call(&h, "place_token", json!({"actor": "Aria Thorne", "x": 4, "y": 9})).await.0);
    assert!(!call(&h, "place_token", json!({"actor": "Goblin A", "x": 14, "y": 4})).await.0);
    assert!(!call(&h, "place_token", json!({"actor": "Goblin B", "x": 16, "y": 5})).await.0);
    assert!(call(&h, "place_token", json!({"actor": "Goblin B", "x": 99, "y": 5})).await.0);
    let (_, state) = call(&h, "get_state", json!({})).await;
    let st: Value = serde_json::from_str(&state).unwrap();
    assert_eq!(st["scene"]["tokens"].as_array().unwrap().len(), 3);

    let (err, order) = call(&h, "start_combat", json!({"actors": ["Goblin A", "Goblin B"], "name": "Chapel ambush"})).await;
    assert!(!err && order.contains("Aria Thorne"), "{order}");
    let (_, atk) = call(&h, "attack", json!({"attacker": "Goblin B", "target": "Aria Thorne"})).await;
    assert!(atk.contains("Goblin B attacks Aria Thorne"), "{atk}");
    assert!(!call(&h, "move_token", json!({"actor": "Goblin B", "x": 15, "y": 6})).await.0);
    let (_, nt) = call(&h, "next_turn", json!({})).await;
    assert!(nt.starts_with("Round"));
    let (_, r) = call(&h, "roll", json!({"expr": "1d20+4", "label": "Goblin perception", "secret": true})).await;
    assert!(r.contains("Goblin perception"));
    let (_, o) = call(&h, "oracle", json!({"question": "Is the crypt door locked?", "likelihood": "likely"})).await;
    assert!(o.starts_with("Oracle (Likely)"));
    assert!(!call(&h, "journal_write", json!({"title": "The Bell That Won’t Stop Ringing", "category": "quest", "body": "Every night…", "objectives": ["Reach the chapel", "Find the bell-ringer"], "active": true})).await.0);
    assert!(!call(&h, "journal_update", json!({"title": "The Bell That Won’t Stop Ringing", "complete": ["reach the chapel"]})).await.0);
    let (_, p) = call(&h, "give_item", json!({"name": "Silver bell", "reason": "found it"})).await;
    assert!(p.starts_with("proposed"));
    let (_, img) = call(&h, "generate_image", json!({"kind": "art", "prompt": "goblins at a brazier"})).await;
    assert!(img.contains("disabled"));

    let w = e.world().unwrap();
    assert!(w.combat.is_some());
    assert!(w.journal[0].objectives[0].done);
    assert_eq!(w.proposals.len(), 1);
    assert!(w.chat.iter().any(|c| matches!(&c.kind, ChatKind::Roll { card } if card.by_gm)));
    assert!(w.chat.iter().any(|c| matches!(&c.kind, ChatKind::Tool { text } if text.contains("secretly"))));
    let rules = call(&h, "rules_lookup", json!({"topic": "encounter budget"})).await.1;
    assert!(rules.contains("Solo rules"), "{rules}");
}
