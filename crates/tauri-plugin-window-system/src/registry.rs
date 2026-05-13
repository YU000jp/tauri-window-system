use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex, Weak};
use std::thread;
use std::time::Duration;

const GEOMETRY_FLUSH_DEBOUNCE: Duration = Duration::from_millis(80);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowDescriptor {
    pub label: String,
    pub url: String,
    pub parent: Option<String>,
    pub title: Option<String>,
    pub geometry: Option<WindowGeometry>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct WindowStateSnapshot {
    windows: HashMap<String, WindowGeometry>,
}

#[derive(Default)]
struct WindowRegistryState {
    windows: HashMap<String, WindowDescriptor>,
    children: HashMap<String, HashSet<String>>,
    reserved: HashSet<String>,
    closing: HashSet<String>,
}

#[derive(Clone)]
pub struct WindowStateStore {
    inner: Arc<WindowStateStoreInner>,
}

struct WindowStateStoreInner {
    path: PathBuf,
    snapshot: Mutex<WindowStateSnapshot>,
    flush_signal: mpsc::SyncSender<()>,
}

impl WindowStateStore {
    pub fn new(path: PathBuf) -> Self {
        let snapshot = load_snapshot(&path).unwrap_or_default();
        let (flush_signal, flush_receiver) = mpsc::sync_channel(1);

        let inner = Arc::new(WindowStateStoreInner {
            path,
            snapshot: Mutex::new(snapshot),
            flush_signal,
        });

        spawn_geometry_flusher(Arc::downgrade(&inner), flush_receiver);

        Self { inner }
    }

    pub fn restore(&self, label: &str) -> Option<WindowGeometry> {
        self.inner
            .snapshot
            .lock()
            .ok()
            .and_then(|snapshot| snapshot.windows.get(label).cloned())
    }

    pub fn remember(&self, label: &str, geometry: WindowGeometry) -> Result<(), String> {
        let mut snapshot = self
            .inner
            .snapshot
            .lock()
            .map_err(|_| "window state store lock poisoned".to_string())?;
        if snapshot.windows.get(label) == Some(&geometry) {
            return Ok(());
        }

        snapshot.windows.insert(label.to_string(), geometry);
        drop(snapshot);
        self.schedule_flush();
        Ok(())
    }

    pub fn forget(&self, label: &str) -> Result<(), String> {
        let mut snapshot = self
            .inner
            .snapshot
            .lock()
            .map_err(|_| "window state store lock poisoned".to_string())?;
        if snapshot.windows.remove(label).is_none() {
            return Ok(());
        }

        drop(snapshot);
        self.schedule_flush();
        Ok(())
    }

    fn schedule_flush(&self) {
        let _ = self.inner.flush_signal.try_send(());
    }

    pub fn flush(&self) -> Result<(), String> {
        self.inner.flush_snapshot()
    }
}

impl WindowStateStoreInner {
    fn flush_snapshot(&self) -> Result<(), String> {
        let snapshot = self
            .snapshot
            .lock()
            .map_err(|_| "window state store lock poisoned".to_string())?
            .clone();

        persist_snapshot(&self.path, &snapshot)
    }
}

impl Drop for WindowStateStoreInner {
    fn drop(&mut self) {
        if let Err(err) = self.flush_snapshot() {
            eprintln!("window-system: final geometry flush failed: {err}");
        }
    }
}

#[derive(Clone)]
pub struct WindowRegistry {
    inner: Arc<WindowRegistryInner>,
}

struct WindowRegistryInner {
    state: Mutex<WindowRegistryState>,
    state_store: WindowStateStore,
}

pub struct WindowReservation {
    registry: WindowRegistry,
    label: String,
    committed: bool,
}

impl WindowReservation {
    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for WindowReservation {
    fn drop(&mut self) {
        if !self.committed {
            self.registry.release_reservation(&self.label);
        }
    }
}

pub struct ClosingGuard {
    registry: WindowRegistry,
    label: String,
}

impl Drop for ClosingGuard {
    fn drop(&mut self) {
        self.registry.finish_closing(&self.label);
    }
}

impl WindowRegistry {
    pub fn new(state_store: WindowStateStore) -> Self {
        Self {
            inner: Arc::new(WindowRegistryInner {
                state: Mutex::new(WindowRegistryState::default()),
                state_store,
            }),
        }
    }

    pub fn reserve_window(
        &self,
        label: &str,
        parent: Option<&str>,
    ) -> Result<WindowReservation, String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "window registry lock poisoned".to_string())?;

        if label.trim().is_empty() {
            return Err("label is required".to_string());
        }

        if state.windows.contains_key(label)
            || state.reserved.contains(label)
            || state.closing.contains(label)
        {
            return Err("window already exists".to_string());
        }

        if let Some(parent) = parent {
            if parent == label {
                return Err("window cannot be its own parent".to_string());
            }

            if !state.windows.contains_key(parent) || state.closing.contains(parent) {
                return Err("parent window not found".to_string());
            }
        }

        state.reserved.insert(label.to_string());

        Ok(WindowReservation {
            registry: self.clone(),
            label: label.to_string(),
            committed: false,
        })
    }

    pub fn insert_reserved(&self, descriptor: WindowDescriptor) -> Result<(), String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "window registry lock poisoned".to_string())?;

        if !state.reserved.remove(&descriptor.label) {
            return Err("window reservation not found".to_string());
        }

        state.update_window(descriptor);
        Ok(())
    }

    pub fn insert(&self, descriptor: WindowDescriptor) -> Result<(), String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "window registry lock poisoned".to_string())?;
        state.update_window(descriptor);
        Ok(())
    }

    pub fn remove(&self, label: &str) -> Result<Option<WindowDescriptor>, String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "window registry lock poisoned".to_string())?;
        Ok(state.remove_window(label))
    }

    pub fn get(&self, label: &str) -> Result<Option<WindowDescriptor>, String> {
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| "window registry lock poisoned".to_string())?;
        Ok(state.windows.get(label).cloned())
    }

    pub fn list(&self) -> Result<Vec<WindowDescriptor>, String> {
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| "window registry lock poisoned".to_string())?;
        let mut list: Vec<_> = state.windows.values().cloned().collect();
        list.sort_by(|left, right| left.label.cmp(&right.label));
        Ok(list)
    }

    // Close cascade only needs labels, so keep this path minimal and index-backed.
    pub fn child_labels_of(&self, parent: &str) -> Result<Vec<String>, String> {
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| "window registry lock poisoned".to_string())?;
        let mut labels: Vec<_> = state
            .children
            .get(parent)
            .into_iter()
            .flat_map(|children| children.iter().cloned())
            .collect();
        labels.sort();
        Ok(labels)
    }

    // Display and diagnostics want full descriptors, so expand labels only here.
    pub fn children_of(&self, parent: &str) -> Result<Vec<WindowDescriptor>, String> {
        let labels = self.child_labels_of(parent)?;
        let mut list = Vec::with_capacity(labels.len());

        for label in labels {
            if let Some(descriptor) = self.get(&label)? {
                list.push(descriptor);
            }
        }

        list.sort_by(|left, right| left.label.cmp(&right.label));
        Ok(list)
    }

    pub fn restore_geometry(&self, label: &str) -> Option<WindowGeometry> {
        self.inner.state_store.restore(label)
    }

    pub fn flush(&self) -> Result<(), String> {
        self.inner.state_store.flush()
    }

    pub fn remember_geometry(&self, label: &str, geometry: WindowGeometry) -> Result<(), String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "window registry lock poisoned".to_string())?;

        let Some(descriptor) = state.windows.get_mut(label) else {
            return Ok(());
        };

        if descriptor.geometry.as_ref() == Some(&geometry) {
            return Ok(());
        }

        descriptor.geometry = Some(geometry.clone());
        drop(state);
        self.inner.state_store.remember(label, geometry)
    }

    pub fn begin_closing(&self, label: &str) -> Result<Option<ClosingGuard>, String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "window registry lock poisoned".to_string())?;

        if state.closing.contains(label) {
            return Ok(None);
        }

        state.closing.insert(label.to_string());
        Ok(Some(ClosingGuard {
            registry: self.clone(),
            label: label.to_string(),
        }))
    }

    fn release_reservation(&self, label: &str) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.reserved.remove(label);
        }
    }

    fn finish_closing(&self, label: &str) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.closing.remove(label);
        }
    }
}

impl WindowRegistryState {
    fn update_window(&mut self, descriptor: WindowDescriptor) {
        let label = descriptor.label.clone();
        let parent = descriptor.parent.clone();

        if let Some(previous) = self.windows.insert(label.clone(), descriptor) {
            self.detach_child(&previous.label, previous.parent.as_deref());
        }

        self.attach_child(&label, parent.as_deref());
    }

    fn remove_window(&mut self, label: &str) -> Option<WindowDescriptor> {
        let descriptor = self.windows.remove(label)?;
        self.detach_child(&descriptor.label, descriptor.parent.as_deref());
        Some(descriptor)
    }

    fn attach_child(&mut self, label: &str, parent: Option<&str>) {
        let Some(parent) = parent else {
            return;
        };

        self.children
            .entry(parent.to_string())
            .or_default()
            .insert(label.to_string());
    }

    fn detach_child(&mut self, label: &str, parent: Option<&str>) {
        let Some(parent) = parent else {
            return;
        };

        if let Some(children) = self.children.get_mut(parent) {
            children.remove(label);
            if children.is_empty() {
                self.children.remove(parent);
            }
        }
    }
}

fn load_snapshot(path: &PathBuf) -> Result<WindowStateSnapshot, String> {
    if !path.exists() {
        return Ok(WindowStateSnapshot::default());
    }

    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| err.to_string())
}

fn persist_snapshot(path: &PathBuf, snapshot: &WindowStateSnapshot) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let raw = serde_json::to_string_pretty(snapshot).map_err(|err| err.to_string())?;
    fs::write(path, raw).map_err(|err| err.to_string())
}

fn spawn_geometry_flusher(
    inner: Weak<WindowStateStoreInner>,
    flush_receiver: mpsc::Receiver<()>,
) {
    thread::spawn(move || {
        while flush_receiver.recv().is_ok() {
            loop {
                match flush_receiver.recv_timeout(GEOMETRY_FLUSH_DEBOUNCE) {
                    Ok(_) => continue,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if let Some(inner) = inner.upgrade() {
                            if let Err(err) = inner.flush_snapshot() {
                                eprintln!(
                                    "window-system: failed to persist window geometry snapshot: {err}"
                                );
                            }
                        } else {
                            break;
                        }
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        if let Some(inner) = inner.upgrade() {
                            if let Err(err) = inner.flush_snapshot() {
                                eprintln!(
                                    "window-system: failed to persist window geometry snapshot: {err}"
                                );
                            }
                        }
                        return;
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be monotonic for tests")
            .as_nanos();
        std::env::temp_dir().join(format!("tauri-window-system-{name}-{suffix}.json"))
    }

    #[test]
    fn remember_and_restore_geometry() {
        let store = WindowStateStore::new(unique_temp_path("geometry"));
        let geometry = WindowGeometry {
            x: 10.0,
            y: 20.0,
            width: 300.0,
            height: 200.0,
        };

        store
            .remember("main", geometry.clone())
            .expect("store should persist geometry");

        assert_eq!(store.restore("main"), Some(geometry));
    }

    #[test]
    fn remember_coalesces_to_latest_persisted_geometry() {
        let path = unique_temp_path("debounce");
        let store = WindowStateStore::new(path.clone());
        let first = WindowGeometry {
            x: 10.0,
            y: 20.0,
            width: 300.0,
            height: 200.0,
        };
        let latest = WindowGeometry {
            x: 40.0,
            y: 50.0,
            width: 640.0,
            height: 480.0,
        };

        store
            .remember("main", first)
            .expect("first geometry should queue");
        store
            .remember("main", latest.clone())
            .expect("latest geometry should queue");

        std::thread::sleep(std::time::Duration::from_millis(150));

        let raw = std::fs::read_to_string(path).expect("debounced snapshot should be written");
        let snapshot: WindowStateSnapshot =
            serde_json::from_str(&raw).expect("snapshot should deserialize");

        assert_eq!(snapshot.windows.get("main"), Some(&latest));
    }

    #[test]
    fn flush_writes_snapshot_immediately() {
        let path = unique_temp_path("flush");
        let store = WindowStateStore::new(path.clone());
        let geometry = WindowGeometry {
            x: 11.0,
            y: 22.0,
            width: 33.0,
            height: 44.0,
        };

        store
            .remember("main", geometry.clone())
            .expect("geometry should queue");
        store.flush().expect("flush should write immediately");

        let raw = std::fs::read_to_string(path).expect("snapshot should exist");
        let snapshot: WindowStateSnapshot =
            serde_json::from_str(&raw).expect("snapshot should deserialize");

        assert_eq!(snapshot.windows.get("main"), Some(&geometry));
    }

    #[test]
    fn remove_does_not_clear_geometry() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("persist")));
        let geometry = WindowGeometry {
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
        };

        registry
            .insert(WindowDescriptor {
                label: "main".into(),
                url: "index.html".into(),
                parent: None,
                title: None,
                geometry: None,
            })
            .expect("main should insert");
        registry
            .remember_geometry("main", geometry.clone())
            .expect("geometry should persist");

        registry.remove("main").expect("remove should work");

        assert_eq!(registry.restore_geometry("main"), Some(geometry));
    }

    #[test]
    fn registry_lists_and_filters_children() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("registry")));
        registry
            .insert(WindowDescriptor {
                label: "main".into(),
                url: "index.html".into(),
                parent: None,
                title: None,
                geometry: None,
            })
            .expect("main should insert");
        registry
            .insert(WindowDescriptor {
                label: "child".into(),
                url: "child.html".into(),
                parent: Some("main".into()),
                title: None,
                geometry: None,
            })
            .expect("child should insert");

        assert_eq!(registry.list().unwrap().len(), 2);
        assert_eq!(registry.children_of("main").unwrap().len(), 1);
        assert_eq!(registry.child_labels_of("main").unwrap(), vec!["child".to_string()]);
    }

    #[test]
    fn update_reindexes_parent_relationships() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("reindex")));
        registry
            .insert(WindowDescriptor {
                label: "main-a".into(),
                url: "main-a.html".into(),
                parent: None,
                title: None,
                geometry: None,
            })
            .expect("main-a should insert");
        registry
            .insert(WindowDescriptor {
                label: "main-b".into(),
                url: "main-b.html".into(),
                parent: None,
                title: None,
                geometry: None,
            })
            .expect("main-b should insert");
        registry
            .insert(WindowDescriptor {
                label: "child".into(),
                url: "child.html".into(),
                parent: Some("main-a".into()),
                title: None,
                geometry: None,
            })
            .expect("child should insert");

        registry
            .insert(WindowDescriptor {
                label: "child".into(),
                url: "child.html".into(),
                parent: Some("main-b".into()),
                title: None,
                geometry: None,
            })
            .expect("child should reinsert with new parent");

        assert!(registry.child_labels_of("main-a").unwrap().is_empty());
        assert_eq!(registry.child_labels_of("main-b").unwrap(), vec!["child".to_string()]);
    }

    #[test]
    fn remove_updates_child_index() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("children")));
        registry
            .insert(WindowDescriptor {
                label: "main".into(),
                url: "index.html".into(),
                parent: None,
                title: None,
                geometry: None,
            })
            .expect("main should insert");
        registry
            .insert(WindowDescriptor {
                label: "child".into(),
                url: "child.html".into(),
                parent: Some("main".into()),
                title: None,
                geometry: None,
            })
            .expect("child should insert");

        registry.remove("child").expect("child should remove");

        assert!(registry.child_labels_of("main").unwrap().is_empty());
        assert!(registry.children_of("main").unwrap().is_empty());
    }

    #[test]
    fn reservation_rejects_duplicate_labels() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("reserve")));
        let _reservation = registry.reserve_window("main", None).expect("first reserve");

        assert!(registry.reserve_window("main", None).is_err());
    }

    #[test]
    fn reservation_rejects_missing_parent() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("parent-missing")));

        assert!(registry.reserve_window("child", Some("main")).is_err());
    }

    #[test]
    fn reservation_rejects_self_parent() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("self-parent")));

        assert!(registry.reserve_window("main", Some("main")).is_err());
    }

    #[test]
    fn reservation_rejects_closing_parent() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("closing-parent")));
        registry
            .insert(WindowDescriptor {
                label: "main".into(),
                url: "index.html".into(),
                parent: None,
                title: None,
                geometry: None,
            })
            .expect("main should insert");
        let guard = registry
            .begin_closing("main")
            .expect("lock should succeed")
            .expect("closing should be marked");

        assert!(registry.reserve_window("child", Some("main")).is_err());
        drop(guard);
    }

    #[test]
    fn closing_guard_prevents_duplicate_begin() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("closing")));
        let guard = registry
            .begin_closing("main")
            .expect("lock should succeed")
            .expect("first closing mark");

        assert!(registry.begin_closing("main").expect("lock should succeed").is_none());

        drop(guard);
        assert!(registry.begin_closing("main").expect("lock should succeed").is_some());
    }
}
