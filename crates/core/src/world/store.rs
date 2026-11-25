//! Worlds on disk: `<root>/worlds/<id>/world.json` + `images/`.
//! Writes are atomic (temp file + rename). Deleted worlds go to `<root>/trash/`
//! so the UI can offer Undo.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::model::World;

#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldSummary {
    pub id: String,
    pub name: String,
    pub genre: String,
    pub hero: String,
    pub session: u32,
    pub updated_at: u64,
    pub cover: Option<String>,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Store { root: root.into() }
    }

    /// `~/.local/share/soloforge` (or the platform equivalent).
    pub fn default_root() -> PathBuf {
        if let Ok(p) = std::env::var("SOLOFORGE_HOME") {
            return PathBuf::from(p);
        }
        directories::ProjectDirs::from("dev", "soloforge", "soloforge")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".soloforge"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn worlds_dir(&self) -> PathBuf {
        self.root.join("worlds")
    }
    pub fn world_dir(&self, id: &str) -> PathBuf {
        self.worlds_dir().join(sanitize(id))
    }
    pub fn images_dir(&self, id: &str) -> PathBuf {
        self.world_dir(id).join("images")
    }

    pub fn save(&self, w: &World) -> Result<()> {
        let dir = self.world_dir(&w.id);
        fs::create_dir_all(dir.join("images"))?;
        let tmp = dir.join("world.json.tmp");
        let data = serde_json::to_vec_pretty(w)?;
        fs::write(&tmp, data)?;
        fs::rename(&tmp, dir.join("world.json"))?;
        Ok(())
    }

    pub fn load(&self, id: &str) -> Result<World> {
        let p = self.world_dir(id).join("world.json");
        let raw = fs::read(&p).with_context(|| format!("reading {}", p.display()))?;
        Ok(serde_json::from_slice(&raw)?)
    }

    pub fn list(&self) -> Vec<WorldSummary> {
        let mut out = vec![];
        let Ok(rd) = fs::read_dir(self.worlds_dir()) else { return out };
        for e in rd.flatten() {
            let id = e.file_name().to_string_lossy().to_string();
            if let Ok(w) = self.load(&id) {
                let hero = w
                    .pc()
                    .map(|p| format!("{}, level {} {}", p.name, p.level, p.class))
                    .unwrap_or_else(|| "No hero yet".into());
                let cover = w.scenes.iter().find_map(|s| s.background.clone());
                out.push(WorldSummary {
                    id: w.id.clone(),
                    name: w.name.clone(),
                    genre: w.tone.first().cloned().unwrap_or_else(|| "Adventure".into()),
                    hero,
                    session: w.session,
                    updated_at: w.updated_at,
                    cover,
                });
            }
        }
        out.sort_by_key(|w| std::cmp::Reverse(w.updated_at));
        out
    }

    /// Move a world to the trash (Undo-able).
    pub fn trash(&self, id: &str) -> Result<()> {
        let from = self.world_dir(id);
        let to = self.root.join("trash").join(sanitize(id));
        fs::create_dir_all(self.root.join("trash"))?;
        if to.exists() {
            fs::remove_dir_all(&to)?;
        }
        fs::rename(from, to)?;
        Ok(())
    }

    pub fn restore(&self, id: &str) -> Result<()> {
        let from = self.root.join("trash").join(sanitize(id));
        fs::create_dir_all(self.worlds_dir())?;
        fs::rename(from, self.world_dir(id))?;
        Ok(())
    }
}

fn sanitize(id: &str) -> String {
    id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_load_list_trash_restore() {
        let dir = std::env::temp_dir().join(format!("sf-store-{}", super::super::model::new_id()));
        let s = Store::new(&dir);
        let w = World::new("The Ashen Vale", "burned valley");
        s.save(&w).unwrap();
        assert_eq!(s.load(&w.id).unwrap(), w);
        assert_eq!(s.list().len(), 1);
        s.trash(&w.id).unwrap();
        assert!(s.list().is_empty());
        s.restore(&w.id).unwrap();
        assert_eq!(s.list()[0].name, "The Ashen Vale");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn ids_cannot_escape_the_root() {
        let s = Store::new("/tmp/x");
        assert_eq!(s.world_dir("../../etc"), PathBuf::from("/tmp/x/worlds/etc"));
    }
}
