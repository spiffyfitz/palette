//! Small application state and crash-recovery snapshots, separate from original
//! user files. A process lease prevents offering another running app's snapshots.
use palette_core::{document::Document, io};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const RECENT_LIMIT: usize = 10;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recovery {
    pub name: String,
    pub original: Option<PathBuf>,
    pub project: String,
    #[serde(skip)]
    pub manifest: PathBuf,
}
pub struct Store {
    root: PathBuf,
    instance: String,
    lease: File,
}

fn json_write(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("Application metadata exceeds 64 KiB".into());
    }
    let mut file = tempfile::NamedTempFile::new_in(path.parent().ok_or("No parent directory")?)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.flush()
        .and_then(|_| file.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
fn json_read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(65537).read_to_end(&mut bytes))
        .map_err(|e| e.to_string())?;
    if bytes.len() > 65536 {
        return Err("Application metadata exceeds 64 KiB".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

impl Store {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(root.join("recovery")).map_err(|e| e.to_string())?;
        let instance = format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos()
        );
        let lease = File::create_new(root.join("recovery").join(format!("{instance}.lock")))
            .map_err(|e| e.to_string())?;
        lease.lock().map_err(|e| e.to_string())?;
        Ok(Self {
            root,
            instance,
            lease,
        })
    }
    pub fn recent(&self) -> Result<Vec<PathBuf>, String> {
        let path = self.root.join("recent.json");
        if !path.exists() {
            return Ok(vec![]);
        }
        let mut paths: Vec<PathBuf> = json_read(&path)?;
        paths.truncate(RECENT_LIMIT);
        Ok(paths)
    }
    pub fn save_recent(&self, paths: &[PathBuf]) -> Result<(), String> {
        json_write(
            &self.root.join("recent.json"),
            &paths.iter().take(RECENT_LIMIT).collect::<Vec<_>>(),
        )
    }
    fn entry(&self, id: u64) -> PathBuf {
        self.root
            .join("recovery")
            .join(format!("{}-{id}.json", self.instance))
    }
    pub fn autosave(
        &self,
        id: u64,
        name: &str,
        original: Option<PathBuf>,
        doc: &Document,
    ) -> Result<(), String> {
        let entry = self.entry(id);
        let project = format!("{}-{id}.palette", self.instance);
        io::save_project(&entry.with_file_name(&project), doc).map_err(|e| e.to_string())?;
        json_write(
            &entry,
            &Recovery {
                name: name.into(),
                original,
                project,
                manifest: entry.clone(),
            },
        )
    }
    pub fn forget(&self, id: u64) -> Result<(), String> {
        let entry = self.entry(id);
        for path in [entry.with_extension("palette"), entry] {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    }
    pub fn discover(&self) -> Result<Vec<Recovery>, String> {
        let mut recoveries = vec![];
        for entry in fs::read_dir(self.root.join("recovery")).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Some((instance, _)) = stem.rsplit_once('-') else {
                continue;
            };
            if instance == self.instance {
                continue;
            }
            let lock = path.with_file_name(format!("{instance}.lock"));
            if let Ok(lease) = File::options().read(true).write(true).open(lock)
                && lease.try_lock().is_err()
            {
                continue;
            }
            if recoveries.len() >= 256 {
                return Err(
                    "More than 256 recovery documents; open older snapshots manually".into(),
                );
            }
            let mut recovery: Recovery = json_read(&path)?;
            if Path::new(&recovery.project)
                .file_name()
                .and_then(|s| s.to_str())
                != Some(&recovery.project)
                || recovery.project != format!("{stem}.palette")
            {
                return Err("Invalid recovery project path".into());
            }
            recovery.manifest = path;
            recoveries.push(recovery);
        }
        recoveries.sort_by(|a, b| a.manifest.cmp(&b.manifest));
        Ok(recoveries)
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        // Keep the lease file as evidence of ownership; the OS releases its lock
        // even after a crash. Discovery ignores leases held by running processes.
        let _ = self.lease.unlock();
    }
}
impl Recovery {
    pub fn project_path(&self) -> PathBuf {
        self.manifest.with_file_name(&self.project)
    }
    pub fn discard(&self) -> Result<(), String> {
        for path in [self.manifest.clone(), self.project_path()] {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    }
}

pub fn data_directory() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("Palette"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|p| PathBuf::from(p).join("Library/Application Support/Palette"))
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state")))
            .map(|p| p.join("palette"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_roundtrips_and_never_offers_another_running_instances_documents() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("original.palette");
        fs::write(&original, b"original sentinel").unwrap();
        let first = Store::new(dir.path().into()).unwrap();
        let mut doc = Document::new(100, 60).unwrap();
        doc.add_layer();
        doc.active_mut().source.set(2, 3, [10, 20, 30, 40]);
        first
            .autosave(7, "Unsaved drawing", Some(original.clone()), &doc)
            .unwrap();
        let second = Store::new(dir.path().into()).unwrap();
        assert!(second.discover().unwrap().is_empty());
        drop(first); // Simulate loss of process ownership; retain its snapshots.
        let entries = second.discover().unwrap();
        assert_eq!(entries.len(), 1);
        assert!(io::open(&entries[0].project_path()).unwrap() == doc);
        assert_eq!(entries[0].original.as_ref(), Some(&original));
        assert_eq!(fs::read(original).unwrap(), b"original sentinel");
        entries[0].discard().unwrap();
        assert!(second.discover().unwrap().is_empty());
    }
    #[test]
    fn recent_metadata_is_bounded_and_recovery_paths_cannot_escape_storage() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().into()).unwrap();
        let paths = vec![PathBuf::from("first.palette"), PathBuf::from("second.png")];
        store.save_recent(&paths).unwrap();
        assert_eq!(store.recent().unwrap(), paths);
        assert!(
            store
                .save_recent(&[PathBuf::from("x".repeat(70000))])
                .is_err()
        );
        assert_eq!(store.recent().unwrap(), paths);
        let entry = dir.path().join("recovery/old-instance-1.json");
        fs::write(
            entry,
            br#"{"name":"bad","original":null,"project":"../../secret.palette"}"#,
        )
        .unwrap();
        assert!(store.discover().is_err());
        store.forget(1234).unwrap();
    }
}
