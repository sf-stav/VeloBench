//! Application state: an in-memory store backed by JSON files on disk.
//!
//! All settings and benchmarks live server-side (never in the browser). The
//! `Store` loads them at boot and writes them back on change. Each mutation is
//! serialised by a per-resource `RwLock`, and persistence snapshots the latest
//! value, so concurrent updates never interleave on disk.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::analyze::SessionAnalysis;
use crate::benchmarks::Benchmark;
use crate::settings::Settings;
use crate::tests::TestDef;

const SETTINGS_FILE: &str = "settings.json";
const BENCHMARKS_FILE: &str = "benchmarks.json";
const ANALYSES_FILE: &str = "analyses.json";
const COMPARISONS_FILE: &str = "comparisons.json";
const TESTS_FILE: &str = "tests.json";
const SESSION_META_FILE: &str = "session_meta.json";

/// A saved comparison of two sessions of the same kind (single or
/// concurrent). The report itself is computed from the two analyses at view
/// time; only the pairing is stored.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionComparison {
    pub id: String,
    /// First (baseline) session id.
    pub a: String,
    /// Second session id.
    pub b: String,
    /// 'single' | 'concurrent' — both sides must match.
    pub kind: String,
    pub created_at: String,
}

/// User-authored session metadata: display name + managed category. Kept in
/// its OWN file (not settings.json — the browser PUTs settings wholesale and
/// would clobber it) and not per-turn (a session's name/category is one fact).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct SessionMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

#[derive(Clone)]
pub struct Store {
    data_dir: PathBuf,
    settings: Arc<RwLock<Settings>>,
    benchmarks: Arc<RwLock<Vec<Benchmark>>>,
    analyses: Arc<RwLock<Vec<SessionAnalysis>>>,
    tests: Arc<RwLock<Vec<TestDef>>>,
    session_meta: Arc<RwLock<HashMap<String, SessionMeta>>>,
    comparisons: Arc<RwLock<Vec<SessionComparison>>>,
}

impl Store {
    /// Create the store and load existing data from `data_dir`.
    pub async fn new(data_dir: PathBuf) -> std::io::Result<Self> {
        tokio::fs::create_dir_all(&data_dir).await?;
        let settings = load_or_default::<Settings>(&data_dir.join(SETTINGS_FILE)).await;
        let benchmarks =
            load_or_default::<Vec<Benchmark>>(&data_dir.join(BENCHMARKS_FILE)).await;
        let analyses = load_or_default::<Vec<SessionAnalysis>>(&data_dir.join(ANALYSES_FILE)).await;
        // Test definitions: user tests from disk, seeded with the built-ins.
        let mut tests = load_or_default::<Vec<TestDef>>(&data_dir.join(TESTS_FILE)).await;
        // Built-ins are code-owned: refresh them on every boot so new fields
        // and behavior propagate (they cannot be edited or deleted in the UI).
        // The user's favorite marks on built-ins survive the refresh.
        let prev_favorite: std::collections::HashMap<String, bool> = tests
            .iter()
            .map(|t| (t.id.clone(), t.favorite))
            .collect();
        tests.retain(|t| !t.prebuilt);
        for pre in crate::tests::prebuilt() {
            let favorite = prev_favorite.get(&pre.id).copied().unwrap_or(false);
            tests.push(TestDef { favorite, ..pre });
        }
        let _ = write_json(&data_dir.join(TESTS_FILE), &tests).await;
        let session_meta: HashMap<String, SessionMeta> =
            load_or_default(&data_dir.join(SESSION_META_FILE)).await;
        let comparisons = load_or_default::<Vec<SessionComparison>>(&data_dir.join(COMPARISONS_FILE)).await;
        Ok(Self {
            data_dir,
            settings: Arc::new(RwLock::new(settings)),
            benchmarks: Arc::new(RwLock::new(benchmarks)),
            analyses: Arc::new(RwLock::new(analyses)),
            tests: Arc::new(RwLock::new(tests)),
            session_meta: Arc::new(RwLock::new(session_meta)),
            comparisons: Arc::new(RwLock::new(comparisons)),
        })
    }

    // ---- Saved session comparisons -------------------------------------

    pub async fn comparisons(&self) -> Vec<SessionComparison> {
        self.comparisons.read().await.clone()
    }

    /// Save a pairing. Both sides must exist and share the same kind
    /// ('concurrent' when the session's turns are concurrent workers).
    pub async fn add_comparison(&self, a: &str, b: &str) -> Result<SessionComparison, String> {
        if a == b {
            return Err("Pick two different sessions.".into());
        }
        let kind_of = |sid: &str, benches: &Vec<Benchmark>| {
            benches
                .iter()
                .find(|x| x.session == sid)
                .map(|x| if x.kind == "concurrent" { "concurrent" } else { "single" })
                .unwrap_or("single")
        };
        let benches = self.benchmarks.read().await;
        let existing: Vec<String> = benches.iter().map(|x| x.session.clone()).collect();
        for sid in [a, b] {
            if !existing.iter().any(|s| s == sid) {
                return Err(format!("Session {sid} has no recorded turns."));
            }
        }
        let ka = kind_of(a, &benches);
        let kb = kind_of(b, &benches);
        drop(benches);
        if ka != kb {
            return Err(format!("Cannot compare a {ka} session with a {kb} session."));
        }
        let c = SessionComparison {
            id: crate::settings::short_id(),
            a: a.into(),
            b: b.into(),
            kind: ka.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        {
            let mut g = self.comparisons.write().await;
            g.push(c.clone());
        }
        let list = self.comparisons().await;
        let _ = write_json(&self.data_dir.join(COMPARISONS_FILE), &list).await;
        Ok(c)
    }

    pub async fn delete_comparison(&self, id: &str) {
        {
            let mut g = self.comparisons.write().await;
            g.retain(|c| c.id != id);
        }
        let list = self.comparisons().await;
        let _ = write_json(&self.data_dir.join(COMPARISONS_FILE), &list).await;
    }

    #[allow(dead_code)]
    /// Reset the database to a fresh state: benchmarks, analyses (sessions)
    /// and settings are wiped; test definitions are KEPT (prebuilts refresh
    /// from code anyway). Used by the "start from scratch" action when the
    /// token-methodology version changes.
    pub async fn wipe_history(&self) {
        {
            let mut s = self.settings.write().await;
            *s = Settings::default();
        }
        {
            let mut b = self.benchmarks.write().await;
            b.clear();
        }
        {
            let mut a = self.analyses.write().await;
            a.clear();
        }
        {
            let mut m = self.session_meta.write().await;
            m.clear();
        }
        self.persist_settings().await;
        let _ = write_json(&self.data_dir.join(SESSION_META_FILE), &HashMap::<String, SessionMeta>::new()).await;
        let _ = write_json(&self.data_dir.join(BENCHMARKS_FILE), &Vec::<Benchmark>::new()).await;
        let _ = write_json(&self.data_dir.join(ANALYSES_FILE), &Vec::<SessionAnalysis>::new()).await;
        tracing::info!("history wiped: benchmarks, analyses and settings cleared (tests kept)");
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    // ---------- settings ----------

    /// Current settings (cloned).
    pub async fn settings(&self) -> Settings {
        self.settings.read().await.clone()
    }

    /// Replace settings, normalise, and persist.
    pub async fn set_settings(&self, mut s: Settings) -> Settings {
        s.normalize();
        *self.settings.write().await = s.clone();
        self.persist_settings().await;
        s
    }

    /// Mutate settings under the lock, then persist.
    #[allow(dead_code)]
    pub async fn update_settings<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut Settings) -> T,
    {
        let mut guard = self.settings.write().await;
        let out = f(&mut guard);
        drop(guard);
        self.persist_settings().await;
        out
    }

    async fn persist_settings(&self) {
        let snapshot = self.settings.read().await.clone();
        let path = self.data_dir.join(SETTINGS_FILE);
        if let Err(e) = write_json(&path, &snapshot).await {
            tracing::warn!(path = %path.display(), "write settings failed: {e}");
        }
    }

    // ---------- tests ----------

    /// All test definitions (built-ins included).
    pub async fn tests(&self) -> Vec<TestDef> {
        self.tests.read().await.clone()
    }

    /// Insert or replace a test definition. Built-ins are immutable.
    pub async fn save_test(&self, t: TestDef) -> Result<(), String> {
        if let Some(existing) = self.tests.read().await.iter().find(|x| x.id == t.id) {
            if existing.prebuilt {
                return Err("Built-in tests cannot be modified.".into());
            }
        }
        crate::tests::validate(&t)?;
        let mut g = self.tests.write().await;
        match g.iter_mut().find(|x| x.id == t.id) {
            Some(slot) => *slot = t,
            None => g.push(t),
        }
        drop(g);
        self.persist_tests().await;
        Ok(())
    }

    /// Delete a test definition. Built-ins are immutable.
    /// Toggle just the favorite flag (index star + top-bar dropdown use).
    pub async fn set_test_favorite(&self, id: &str, favorite: bool) -> Result<(), String> {
        let mut g = self.tests.write().await;
        let t = g
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| "unknown test".to_string())?;
        t.favorite = favorite;
        drop(g);
        let tests = self.tests().await;
        let _ = write_json(&self.data_dir.join(TESTS_FILE), &tests).await;
        Ok(())
    }

    pub async fn delete_test(&self, id: &str) -> Result<(), String> {
        {
            let g = self.tests.read().await;
            match g.iter().find(|x| x.id == id) {
                Some(t) if t.prebuilt => {
                    return Err("Built-in tests cannot be deleted.".into())
                }
                None => return Err("No such test.".into()),
                _ => {}
            }
        }
        let mut g = self.tests.write().await;
        g.retain(|x| x.id != id);
        drop(g);
        self.persist_tests().await;
        Ok(())
    }

    async fn persist_tests(&self) {
        let snapshot = self.tests.read().await.clone();
        let path = self.data_dir.join(TESTS_FILE);
        if let Err(e) = write_json(&path, &snapshot).await {
            tracing::warn!(path = %path.display(), "write tests failed: {e}");
        }
    }

    // ---------- benchmarks ----------

    pub async fn benchmarks(&self) -> Vec<Benchmark> {
        self.benchmarks.read().await.clone()
    }

    pub async fn benchmark(&self, id: &str) -> Option<Benchmark> {
        self.benchmarks.read().await.iter().find(|b| b.id == id).cloned()
    }

    pub async fn add_benchmark(&self, b: Benchmark) -> Benchmark {
        let mut g = self.benchmarks.write().await;
        g.push(b.clone());
        drop(g);
        self.persist_benchmarks().await;
        b
    }

    pub async fn delete_benchmark(&self, id: &str) -> bool {
        let mut g = self.benchmarks.write().await;
        let before = g.len();
        g.retain(|b| b.id != id);
        let changed = g.len() != before;
        drop(g);
        if changed {
            self.persist_benchmarks().await;
        }
        changed
    }

    async fn persist_benchmarks(&self) {
        let snapshot = self.benchmarks.read().await.clone();
        let path = self.data_dir.join(BENCHMARKS_FILE);
        if let Err(e) = write_json(&path, &snapshot).await {
            tracing::warn!(path = %path.display(), "write benchmarks failed: {e}");
        }
    }

    /// Replace one record in place (used to persist analysis results).
    pub async fn update_benchmark(&self, updated: Benchmark) -> bool {
        let mut g = self.benchmarks.write().await;
        let mut changed = false;
        for b in g.iter_mut() {
            if b.id == updated.id {
                *b = updated.clone();
                changed = true;
                break;
            }
        }
        drop(g);
        if changed {
            self.persist_benchmarks().await;
        }
        changed
    }

    /// Replace several records in place with a single persistence pass
    /// (used by regime backfill so a boot-time sweep writes the file once).
    pub async fn update_benchmarks<I>(&self, updates: I) -> usize
    where
        I: IntoIterator<Item = Benchmark>,
    {
        let mut g = self.benchmarks.write().await;
        let mut changed = 0usize;
        for updated in updates {
            for b in g.iter_mut() {
                if b.id == updated.id {
                    *b = updated;
                    changed += 1;
                    break;
                }
            }
        }
        drop(g);
        if changed > 0 {
            self.persist_benchmarks().await;
        }
        changed
    }

    // ---------- session meta (custom name + category) ----------

    /// All session metadata (cloned).
    pub async fn session_meta(&self) -> HashMap<String, SessionMeta> {
        self.session_meta.read().await.clone()
    }

    /// Replace one session's metadata entry and persist. An entry with neither
    /// name nor category removes the key entirely.
    pub async fn set_session_meta(&self, session: &str, meta: SessionMeta) -> SessionMeta {
        let mut g = self.session_meta.write().await;
        let stored = if meta.name.is_none() && meta.category.is_none() {
            g.remove(session);
            SessionMeta::default()
        } else {
            let entry = g.entry(session.to_string()).or_default();
            *entry = meta;
            entry.clone()
        };
        drop(g);
        self.persist_session_meta().await;
        stored
    }

    /// Re-point every session from one managed category to another (rename).
    pub async fn rename_session_category(&self, from: &str, to: &str) {
        {
            let mut g = self.session_meta.write().await;
            for meta in g.values_mut() {
                if meta.category.as_deref() == Some(from) {
                    meta.category = Some(to.to_string());
                }
            }
        }
        self.persist_session_meta().await;
    }

    /// Drop a managed category from every session (category was removed).
    pub async fn scrub_session_category(&self, category: &str) {
        {
            let mut g = self.session_meta.write().await;
            for meta in g.values_mut() {
                if meta.category.as_deref() == Some(category) {
                    meta.category = None;
                }
            }
            g.retain(|_, m| m.name.is_some() || m.category.is_some());
        }
        self.persist_session_meta().await;
    }

    async fn persist_session_meta(&self) {
        let snapshot = self.session_meta.read().await.clone();
        let path = self.data_dir.join(SESSION_META_FILE);
        if let Err(e) = write_json(&path, &snapshot).await {
            tracing::warn!(path = %path.display(), "write session meta failed: {e}");
        }
    }

    // ---------- session analyses ----------

    pub async fn analyses(&self) -> Vec<SessionAnalysis> {
        self.analyses.read().await.clone()
    }

    pub async fn analysis(&self, session: &str) -> Option<SessionAnalysis> {
        self.analyses
            .read()
            .await
            .iter()
            .find(|a| a.session == session)
            .cloned()
    }

    /// Insert or replace the analysis for a session, then persist.
    pub async fn upsert_analysis(&self, a: SessionAnalysis) {
        let mut g = self.analyses.write().await;
        match g.iter_mut().find(|x| x.session == a.session) {
            Some(slot) => *slot = a.clone(),
            None => g.push(a.clone()),
        }
        drop(g);
        self.persist_analyses().await;
    }

    /// Apply a mutation to a session's analysis only while it is still
    /// `running` (atomic under the lock). Progress reports from worker tasks
    /// go through here so they can never overwrite a terminal state.
    pub async fn update_analysis_if_running<F>(&self, session: &str, f: F)
    where
        F: FnOnce(SessionAnalysis) -> SessionAnalysis,
    {
        let mut g = self.analyses.write().await;
        let Some(slot) = g.iter_mut().find(|a| a.session == session) else {
            return;
        };
        if slot.status != "running" {
            return;
        }
        *slot = f(slot.clone());
        drop(g);
        self.persist_analyses().await;
    }

    async fn persist_analyses(&self) {
        let snapshot = self.analyses.read().await.clone();
        let path = self.data_dir.join(ANALYSES_FILE);
        if let Err(e) = write_json(&path, &snapshot).await {
            tracing::warn!(path = %path.display(), "write analyses failed: {e}");
        }
    }

    // ---------- deterministic regime stamping ----------

    /// Classify a finished record off the hot path (background thread) and
    /// persist the labels. Generation itself is never delayed: the record is
    /// already in the store when this runs.
    pub fn spawn_stamp(&self, b: Benchmark) {
        let store = self.clone();
        tokio::spawn(async move {
            let res = tokio::task::spawn_blocking(move || {
                let mut b = b;
                let changed = crate::freetier::stamp_record(&mut b, false);
                (b, changed)
            })
            .await;
            match res {
                Ok((b, true)) => {
                    store.update_benchmark(b).await;
                }
                Ok((_, false)) => {}
                Err(e) => tracing::warn!("regime stamp task failed: {e}"),
            }
        });
    }

    /// Boot-time backfill: stamp deterministic regimes onto records that
    /// predate the classifier (events without labels). `only_missing` keeps
    /// labels from prior helper analyses intact, so classifier improvements
    /// apply to history without erasing refined regimes.
    pub fn spawn_backfill(&self) {
        let store = self.clone();
        tokio::spawn(async move {
            let todo: Vec<Benchmark> =
                store.benchmarks().await.into_iter().filter(crate::freetier::needs_stamp).collect();
            if todo.is_empty() {
                return;
            }
            let n = todo.len();
            tracing::info!("regime backfill: classifying {n} record(s)");
            let res = tokio::task::spawn_blocking(move || {
                let mut out = Vec::new();
                for mut b in todo {
                    if crate::freetier::stamp_record(&mut b, true) {
                        out.push(b);
                    }
                }
                out
            })
            .await;
            match res {
                Ok(list) => {
                    let k = list.len();
                    store.update_benchmarks(list).await;
                    tracing::info!("regime backfill stamped {k}/{n} record(s)");
                }
                Err(e) => tracing::warn!("regime backfill failed: {e}"),
            }
        });
    }
}

async fn load_or_default<T>(path: &std::path::Path) -> T
where
    T: serde::de::DeserializeOwned + Default,
{
    match tokio::fs::read_to_string(path).await {
        Ok(text) => match serde_json::from_str::<T>(&text) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(path = %path.display(), "failed to parse, using default: {e}");
                T::default()
            }
        },
        Err(_) => T::default(),
    }
}

async fn write_json(path: &std::path::Path, v: &impl serde::Serialize) -> std::io::Result<()> {
    let bytes = serde_json::to_vec_pretty(v).map_err(std::io::Error::other)?;
    tokio::fs::write(path, bytes).await?;
    Ok(())
}
