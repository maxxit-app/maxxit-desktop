use crate::storage::Store;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
};

const MAX_ENTRIES: usize = 50_000;
#[derive(Default)]
pub struct Sources {
    pub codex: Option<PathBuf>,
    pub claude: Option<PathBuf>,
}
impl Sources {
    pub fn local() -> Self {
        let home = dirs::home_dir();
        Self {
            codex: std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .or_else(|| home.as_ref().map(|h| h.join(".codex"))),
            claude: std::env::var_os("CLAUDE_CONFIG_DIR")
                .map(PathBuf::from)
                .or_else(|| home.map(|h| h.join(".claude"))),
        }
    }
}
#[derive(Default)]
struct Discovery {
    names: BTreeMap<String, String>,
    roots: BTreeMap<String, String>,
}
impl Discovery {
    fn root(&mut self, root: &str) {
        let path = Path::new(root);
        if !path.is_absolute() || self.roots.contains_key(root) {
            return;
        }
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            let key = format!("root:{root}");
            self.names
                .entry(key.clone())
                .or_insert_with(|| name.to_owned());
            self.roots.insert(root.to_owned(), key);
        }
    }
}
fn plain_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
}
fn plain_file(path: &Path) -> Option<fs::File> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return None;
    }
    fs::File::open(path).ok()
}
fn read_json(path: &Path) -> Option<Value> {
    let file = plain_file(path)?;
    let mut bytes = Vec::new();
    file.take(8_000_001).read_to_end(&mut bytes).ok()?;
    if bytes.len() > 8_000_000 {
        return None;
    }
    serde_json::from_slice(&bytes).ok()
}
fn registry(root: &Path, found: &mut Discovery) {
    if !plain_directory(root) {
        return;
    }
    let Some(state) = read_json(&root.join(".codex-global-state.json")) else {
        return;
    };
    if let Some(projects) = state["local-projects"].as_object() {
        for (id, project) in projects.iter().take(MAX_ENTRIES) {
            let Some(name) = project["name"].as_str().filter(|n| !n.trim().is_empty()) else {
                continue;
            };
            let key = format!("project:{id}");
            found.names.insert(key.clone(), name.to_owned());
            if let Some(roots) = project["rootPaths"].as_array() {
                for root in roots
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|r| Path::new(r).is_absolute())
                {
                    found.roots.insert(root.to_owned(), key.clone());
                }
            }
        }
    }
    for key in ["electron-saved-workspace-roots", "active-workspace-roots"] {
        if let Some(roots) = state[key].as_array() {
            for root in roots.iter().take(MAX_ENTRIES).filter_map(Value::as_str) {
                found.root(root);
            }
        }
    }
}
fn session_root(path: &Path, codex: bool) -> Option<String> {
    let file = plain_file(path)?;
    // Bound reads even when a transcript contains a huge message. Keep only cwd.
    for line in BufReader::new(file.take(256_000))
        .lines()
        .take(64)
        .flatten()
    {
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let cwd = if codex {
            if record["type"] != "session_meta" {
                continue;
            }
            record["payload"]["cwd"].as_str()
        } else {
            record["cwd"].as_str()
        };
        if let Some(cwd) = cwd.filter(|cwd| Path::new(cwd).is_absolute()) {
            return Some(cwd.to_owned());
        }
    }
    None
}
fn codex_sessions(root: &Path, depth: usize, examined: &mut usize, found: &mut Discovery) {
    if depth > 4 || !plain_directory(root) {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if *examined >= MAX_ENTRIES {
            break;
        }
        *examined += 1;
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if kind.is_dir() && name.chars().all(|c| c.is_ascii_digit()) {
            codex_sessions(&entry.path(), depth + 1, examined, found);
        } else if kind.is_file() && name.starts_with("rollout-") && name.ends_with(".jsonl") {
            if let Some(cwd) = session_root(&entry.path(), true) {
                found.root(&cwd);
            }
        }
    }
}
fn claude_projects(root: &Path, found: &mut Discovery) {
    if !plain_directory(root) || !plain_directory(&root.join("projects")) {
        return;
    }
    let Ok(entries) = fs::read_dir(root.join("projects")) else {
        return;
    };
    let mut examined = 0;
    for entry in entries.flatten().take(MAX_ENTRIES) {
        if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            continue;
        }
        let folder = entry.path();
        let index = read_json(&folder.join("sessions-index.json"));
        if let Some(index) = index {
            if let Some(root) = index["originalPath"].as_str() {
                found.root(root);
            }
            if let Some(sessions) = index["entries"].as_array() {
                for session in sessions.iter().take(MAX_ENTRIES) {
                    if let Some(root) = session["projectPath"].as_str() {
                        found.root(root);
                    }
                }
            }
        }
        // Current Claude versions need not write an index. Never decode folder slugs.
        let Ok(sessions) = fs::read_dir(&folder) else {
            continue;
        };
        for session in sessions.flatten() {
            if examined >= MAX_ENTRIES {
                return;
            }
            examined += 1;
            if !session.file_type().is_ok_and(|kind| kind.is_file())
                || session.path().extension().is_none_or(|ext| ext != "jsonl")
            {
                continue;
            }
            if let Some(cwd) = session_root(&session.path(), false) {
                found.root(&cwd);
                break;
            }
        }
    }
}
fn project_record(source: &str, name: &str) -> Value {
    let digest = Sha256::digest(format!("maxxit:project:{source}").as_bytes());
    let mut bytes: [u8; 16] = digest[..16].try_into().unwrap();
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let mut name = name.trim().to_owned();
    while name.len() > 120 {
        name.pop();
    }
    json!({"id":uuid::Uuid::from_bytes(bytes).to_string(),"name":name,"description":"","createdAt":chrono::Utc::now().to_rfc3339()})
}
pub fn import_local(store: &Store, sources: &Sources, now: i64) -> Result<(), String> {
    let last_scan = store.get("projectDiscoveryAt")?.and_then(|v| v.as_i64());
    if last_scan.is_some_and(|last| now >= last && now - last < 60) {
        return Ok(());
    }
    let mut found = Discovery::default();
    if let Some(root) = &sources.codex {
        registry(root, &mut found);
        codex_sessions(&root.join("sessions"), 0, &mut 0, &mut found);
        codex_sessions(&root.join("archived_sessions"), 0, &mut 0, &mut found);
    }
    if let Some(root) = &sources.claude {
        claude_projects(root, &mut found);
    }
    let existing = store.projects()?;
    let dismissed = store.get("dismissedProviderProjects")?.unwrap_or(json!([]));
    let mut source_ids = store.get("providerProjectIds")?.unwrap_or(json!({}));
    let previous_ids = source_ids.clone();
    for (source, name) in found.names {
        let mut aliases = vec![source.clone()];
        aliases.extend(
            found
                .roots
                .iter()
                .filter(|(_, key)| **key == source)
                .map(|(root, _)| format!("root:{root}")),
        );
        let aliases: Vec<String> = aliases
            .iter()
            .map(|key| format!("{:x}", Sha256::digest(key.as_bytes())))
            .collect();
        let mut project = project_record(&source, &name);
        if let Some(id) = aliases.iter().find_map(|key| source_ids[key].as_str()) {
            project["id"] = json!(id);
        }
        for alias in aliases {
            source_ids[alias] = project["id"].clone();
        }
        if dismissed
            .as_array()
            .is_some_and(|ids| ids.contains(&project["id"]))
        {
            continue;
        }
        if let Some(saved) = existing.iter().find(|saved| saved["id"] == project["id"]) {
            if saved["name"] != project["name"] {
                let mut updated = saved.clone();
                updated["name"] = project["name"].clone();
                store.save_project(&updated)?;
            }
        } else {
            store.save_project(&project)?;
        }
    }
    if source_ids != previous_ids {
        store.set("providerProjectIds", &source_ids)?;
    }
    store.set("projectDiscoveryAt", &json!(now))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        directory: PathBuf,
        sources: Sources,
        store: Store,
    }
    impl Fixture {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
            let codex = directory.join("codex");
            let claude = directory.join("claude");
            fs::create_dir_all(&codex).unwrap();
            fs::create_dir_all(&claude).unwrap();
            let store = Store::open(&directory.join("store.sqlite")).unwrap();
            Self {
                directory,
                sources: Sources {
                    codex: Some(codex),
                    claude: Some(claude),
                },
                store,
            }
        }
        fn write(&self, provider: &str, path: &str, text: &str) {
            let root = if provider == "codex" {
                &self.sources.codex
            } else {
                &self.sources.claude
            };
            let path = root.as_ref().unwrap().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        fn scan(&self, now: i64) {
            import_local(&self.store, &self.sources, now).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).unwrap();
        }
    }
    #[test]
    fn discovers_registry_cli_and_claude_with_cross_provider_deduplication() {
        let fixture = Fixture::new();
        fixture.write("codex", ".codex-global-state.json", r#"{"local-projects":{"one":{"name":"Named workspace","rootPaths":["/private/shared"]},"two":{"name":"Named workspace","rootPaths":["/private/other"]}}}"#);
        fixture.write(
            "codex",
            "sessions/2026/10/08/rollout-one.jsonl",
            "{\"type\":\"session_meta\",\"payload\":{\"cwd\":\"/private/cli-only\"}}\n",
        );
        fixture.write(
            "claude",
            "projects/encoded/sessions-index.json",
            r#"{"originalPath":"/private/shared","entries":[{"projectPath":"/private/shared"}]}"#,
        );
        fixture.write("claude", "projects/no-index/session.jsonl", "malformed\n{\"type\":\"user\",\"cwd\":\"/private/claude-only\",\"message\":\"PRIVATE_PROMPT\"}\n");
        fixture.scan(100);
        let projects = fixture.store.projects().unwrap();
        assert_eq!(projects.len(), 4);
        let text = serde_json::to_string(&projects).unwrap();
        assert!(!text.contains("/private/"));
        assert!(!text.contains("PRIVATE_PROMPT"));
        assert!(fixture
            .store
            .sync_batch(&crate::model::Settings::default())
            .unwrap()
            .is_empty());
        let shared = crate::model::Settings {
            project_sync: true,
            ..Default::default()
        };
        assert_eq!(fixture.store.sync_batch(&shared).unwrap().len(), 4);
    }
    #[test]
    fn refresh_preserves_description_archive_and_deletions_and_follows_renames() {
        let fixture = Fixture::new();
        fixture.write(
            "codex",
            ".codex-global-state.json",
            r#"{"local-projects":{"one":{"name":"Before"}}}"#,
        );
        fixture.scan(100);
        let mut project = fixture.store.projects().unwrap()[0].clone();
        project["description"] = json!("My goal");
        project["archived"] = json!(true);
        fixture.store.save_project(&project).unwrap();
        fixture.write(
            "codex",
            ".codex-global-state.json",
            r#"{"local-projects":{"one":{"name":"After"}}}"#,
        );
        fixture.scan(101);
        assert_eq!(fixture.store.projects().unwrap()[0]["name"], "Before");
        fixture.scan(161);
        project["name"] = json!("After");
        assert_eq!(fixture.store.projects().unwrap()[0], project);
        fixture
            .store
            .remove_project(project["id"].as_str().unwrap())
            .unwrap();
        fixture.scan(222);
        assert!(fixture.store.projects().unwrap().is_empty());
    }
    #[test]
    fn saved_workspace_keeps_the_identity_discovered_from_a_cli_session() {
        let fixture = Fixture::new();
        fixture.write(
            "codex",
            "sessions/rollout-one.jsonl",
            r#"{"type":"session_meta","payload":{"cwd":"/private/shared"}}"#,
        );
        fixture.scan(100);
        let id = fixture.store.projects().unwrap()[0]["id"].clone();
        fixture.write(
            "codex",
            ".codex-global-state.json",
            r#"{"local-projects":{"one":{"name":"Saved name","rootPaths":["/private/shared"]}}}"#,
        );
        fixture.scan(161);
        let projects = fixture.store.projects().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0]["id"], id);
        assert_eq!(projects[0]["name"], "Saved name");
        assert!(!fixture
            .store
            .get("providerProjectIds")
            .unwrap()
            .unwrap()
            .to_string()
            .contains("/private/"));
    }
    #[test]
    fn legacy_roots_missing_malformed_files_and_symlinks_are_safe() {
        let fixture = Fixture::new();
        fixture.write(
            "codex",
            ".codex-global-state.json",
            r#"{"electron-saved-workspace-roots":["/private/example","/private/example"]}"#,
        );
        fixture.write("claude", "projects/broken/sessions-index.json", "invalid");
        fixture.write("claude", "projects/broken/session.jsonl", "invalid");
        std::os::unix::fs::symlink(
            fixture.sources.codex.as_ref().unwrap(),
            fixture
                .sources
                .claude
                .as_ref()
                .unwrap()
                .join("projects/link"),
        )
        .unwrap();
        fixture.scan(100);
        assert_eq!(fixture.store.projects().unwrap().len(), 1);
        fixture.write("codex", ".codex-global-state.json", "invalid");
        fixture.scan(161);
        assert_eq!(fixture.store.projects().unwrap().len(), 1);
    }
}
