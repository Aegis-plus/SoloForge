//! SoloForge core: the table (world + rules), the MCP server the GM plays
//! through, the Claude Code GM runner and the image client. No Tauri here,
//! so blaze/cargo can check and test it headless.

pub mod art;
pub mod engine;
pub mod gm;
pub mod mcp;
pub mod rules;
pub mod settings;
pub mod world;

pub use engine::{Engine, EventSink, NewWorld, NullSink};
pub use settings::Settings;
