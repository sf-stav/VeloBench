//! SQLite-backed store for recorded turns (benchmark records).
//!
//! Every turn is one row: a compact summary (`turns`, everything the Sessions
//! list needs) and the full record (`turn_records`: prompt, output, every
//! token event) kept in a separate table so listing sessions never touches
//! the large payloads. Nothing is held in memory; reads fetch only the rows a
//! request needs, and finishing a turn is a single INSERT — resident memory
//! stays flat no matter how many sessions are saved.
//!
//! Older versions kept every turn in one `benchmarks.json`, loaded whole into
//! memory and rewritten whole after every turn. That file is migrated here on
//! first boot (streamed record by record, so the migration itself never holds
//! the history in memory) and then renamed to `benchmarks.json.migrated` —
//! kept as a backup, never deleted.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection, OptionalExtension};

use crate::benchmarks::Benchmark;
use crate::sessions_view::TurnSummary;

const DB_FILE: &str = "velobench.db";
const LEGACY_FILE: &str = "benchmarks.json";
const LEGACY_DONE: &str = "benchmarks.json.migrated";
/// Rows per transaction during the one-time migration.
const MIGRATE_BATCH: usize = 200;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS turns (
    id         TEXT PRIMARY KEY,
    session    TEXT NOT NULL,
    seq        INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    kind       TEXT NOT NULL,
    unstamped  INTEGER NOT NULL DEFAULT 0,
    summary    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS turns_by_session ON turns(session, created_at, seq);
CREATE INDEX IF NOT EXISTS turns_by_seq ON turns(seq);
CREATE INDEX IF NOT EXISTS turns_unstamped ON turns(seq) WHERE unstamped = 1;
CREATE TABLE IF NOT EXISTS turn_records (
    id     TEXT PRIMARY KEY,
    record TEXT NOT NULL
);
";

#[derive(Clone)]
pub struct TurnStore {
    /// Single writer connection (SQLite allows one writer at a time anyway).
    writer: Arc<Mutex<Connection>>,
    /// Reader connection: WAL mode lets reads proceed during a write.
    reader: Arc<Mutex<Connection>>,
}

fn open_conn(path: &Path) -> rusqlite::Result<Connection> {
    let c = Connection::open(path)?;
    c.busy_timeout(std::time::Duration::from_secs(30))?;
    c.pragma_update(None, "journal_mode", "WAL")?;
    c.pragma_update(None, "synchronous", "NORMAL")?;
    c.pragma_update(None, "foreign_keys", "OFF")?;
    Ok(c)
}

fn io(e: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(e.to_string())
}

/// Insert or replace one turn. A replaced turn keeps its insertion position.
fn upsert(tx: &Connection, b: &Benchmark) -> rusqlite::Result<()> {
    let seq: i64 = match tx
        .query_row("SELECT seq FROM turns WHERE id = ?1", [&b.id], |r| r.get(0))
        .optional()?
    {
        Some(s) => s,
        None => tx.query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM turns", [], |r| r.get(0))?,
    };
    let summary = TurnSummary::from_benchmark(b, seq as u64);
    let summary_json = serde_json::to_string(&summary).map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
    let record_json = serde_json::to_string(b).map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
    tx.execute(
        "INSERT INTO turns (id, session, seq, created_at, kind, unstamped, summary)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET session = excluded.session,
             created_at = excluded.created_at, kind = excluded.kind,
             unstamped = excluded.unstamped, summary = excluded.summary",
        params![b.id, b.session, seq, b.created_at, b.kind, summary.unstamped as i64, summary_json],
    )?;
    tx.execute(
        "INSERT INTO turn_records (id, record) VALUES (?1, ?2)
         ON CONFLICT(id) DO UPDATE SET record = excluded.record",
        params![b.id, record_json],
    )?;
    Ok(())
}

/// Atomic file write: temp file in the same directory, then rename over.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

fn parse_record(id_hint: &str, json: &str) -> Option<Benchmark> {
    match serde_json::from_str::<Benchmark>(json) {
        Ok(b) => Some(b),
        Err(e) => {
            tracing::warn!("unreadable turn record {id_hint}: {e}");
            None
        }
    }
}

impl TurnStore {
    /// Open (or create) the database and migrate a legacy `benchmarks.json`
    /// if one is present. Blocking work runs off the async runtime.
    pub async fn open(data_dir: PathBuf) -> std::io::Result<Self> {
        let (writer, reader) = tokio::task::spawn_blocking(move || -> std::io::Result<(Connection, Connection)> {
            let path = data_dir.join(DB_FILE);
            let mut w = open_conn(&path).map_err(io)?;
            w.execute_batch(SCHEMA).map_err(io)?;
            migrate_legacy(&data_dir, &mut w);
            let r = open_conn(&path).map_err(io)?;
            let (turns, sessions): (i64, i64) = r
                .query_row("SELECT COUNT(*), COUNT(DISTINCT session) FROM turns", [], |x| Ok((x.get(0)?, x.get(1)?)))
                .map_err(io)?;
            tracing::info!(turns, sessions, db = %path.display(), "turn store ready");
            Ok((w, r))
        })
        .await
        .map_err(io)??;
        Ok(Self { writer: Arc::new(Mutex::new(writer)), reader: Arc::new(Mutex::new(reader)) })
    }

    async fn read<T, F>(&self, f: F) -> Option<T>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        let conn = self.reader.clone();
        let res = tokio::task::spawn_blocking(move || {
            let c = conn.lock().unwrap_or_else(|p| p.into_inner());
            f(&c)
        })
        .await;
        match res {
            Ok(Ok(v)) => Some(v),
            Ok(Err(e)) => {
                tracing::warn!("turn store read failed: {e}");
                None
            }
            Err(e) => {
                tracing::warn!("turn store read task failed: {e}");
                None
            }
        }
    }

    async fn write<T, F>(&self, f: F) -> Option<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        let conn = self.writer.clone();
        let res = tokio::task::spawn_blocking(move || {
            let mut c = conn.lock().unwrap_or_else(|p| p.into_inner());
            f(&mut c)
        })
        .await;
        match res {
            Ok(Ok(v)) => Some(v),
            Ok(Err(e)) => {
                tracing::warn!("turn store write failed: {e}");
                None
            }
            Err(e) => {
                tracing::warn!("turn store write task failed: {e}");
                None
            }
        }
    }

    // ---------- reads ----------

    /// Every turn summary, in insertion order.
    pub async fn summaries(&self) -> Vec<TurnSummary> {
        self.read(|c| {
            let mut st = c.prepare("SELECT summary FROM turns ORDER BY seq")?;
            let rows = st.query_map([], |r| r.get::<_, String>(0))?;
            let mut out = Vec::new();
            for s in rows {
                if let Ok(t) = serde_json::from_str::<TurnSummary>(&s?) {
                    out.push(t);
                }
            }
            Ok(out)
        })
        .await
        .unwrap_or_default()
    }

    /// Whether any turn of this session exists.
    pub async fn has_session(&self, session: &str) -> bool {
        let s = session.to_string();
        self.read(move |c| {
            c.query_row("SELECT EXISTS(SELECT 1 FROM turns WHERE session = ?1)", [&s], |r| r.get::<_, bool>(0))
        })
        .await
        .unwrap_or(false)
    }

    /// Kind of a session's first recorded turn (insertion order).
    pub async fn session_kind(&self, session: &str) -> Option<String> {
        let s = session.to_string();
        self.read(move |c| {
            c.query_row("SELECT kind FROM turns WHERE session = ?1 ORDER BY seq LIMIT 1", [&s], |r| r.get(0))
                .optional()
        })
        .await
        .flatten()
    }

    /// Full records of one session, oldest first.
    pub async fn session_turns(&self, session: &str) -> Vec<Benchmark> {
        let s = session.to_string();
        self.read(move |c| {
            let mut st = c.prepare(
                "SELECT t.id, r.record FROM turns t JOIN turn_records r ON r.id = t.id
                 WHERE t.session = ?1 ORDER BY t.created_at, t.seq",
            )?;
            let rows = st.query_map([&s], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            let mut out = Vec::new();
            for row in rows {
                let (id, json) = row?;
                if let Some(b) = parse_record(&id, &json) {
                    out.push(b);
                }
            }
            Ok(out)
        })
        .await
        .unwrap_or_default()
    }

    /// One full record by id.
    pub async fn get(&self, id: &str) -> Option<Benchmark> {
        let id = id.to_string();
        let json = self
            .read(move |c| {
                c.query_row("SELECT record FROM turn_records WHERE id = ?1", [&id], |r| r.get::<_, String>(0))
                    .optional()
            })
            .await
            .flatten()?;
        parse_record("", &json)
    }

    /// Raw JSON records with seq > `after`, in insertion order, at most
    /// `limit` of them (keyset pagination for streaming exports).
    pub async fn raw_records_after(&self, after: i64, limit: usize) -> Vec<(i64, String)> {
        self.read(move |c| {
            let mut st = c.prepare(
                "SELECT t.seq, r.record FROM turns t JOIN turn_records r ON r.id = t.id
                 WHERE t.seq > ?1 ORDER BY t.seq LIMIT ?2",
            )?;
            let rows = st.query_map(params![after, limit as i64], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect()
        })
        .await
        .unwrap_or_default()
    }

    /// Ids of turns whose token events still lack regime labels.
    pub async fn unstamped_ids(&self) -> Vec<String> {
        self.read(|c| {
            let mut st = c.prepare("SELECT id FROM turns WHERE unstamped = 1 ORDER BY seq")?;
            let rows = st.query_map([], |r| r.get(0))?;
            rows.collect()
        })
        .await
        .unwrap_or_default()
    }

    // ---------- writes ----------

    /// Insert (or replace, by id) a turn.
    pub async fn put(&self, b: Benchmark) -> bool {
        self.write(move |c| {
            let tx = c.transaction()?;
            upsert(&tx, &b)?;
            tx.commit()
        })
        .await
        .is_some()
    }

    /// Replace an existing turn; returns false when the id is unknown.
    pub async fn replace(&self, b: Benchmark) -> bool {
        self.write(move |c| {
            let tx = c.transaction()?;
            let exists: bool =
                tx.query_row("SELECT EXISTS(SELECT 1 FROM turns WHERE id = ?1)", [&b.id], |r| r.get(0))?;
            if exists {
                upsert(&tx, &b)?;
            }
            tx.commit()?;
            Ok(exists)
        })
        .await
        .unwrap_or(false)
    }

    pub async fn delete(&self, id: &str) -> bool {
        let id = id.to_string();
        self.write(move |c| {
            let tx = c.transaction()?;
            let n = tx.execute("DELETE FROM turns WHERE id = ?1", [&id])?;
            tx.execute("DELETE FROM turn_records WHERE id = ?1", [&id])?;
            tx.commit()?;
            Ok(n > 0)
        })
        .await
        .unwrap_or(false)
    }

    /// Remove every turn (the "start from scratch" wipe) and give the space
    /// back to the filesystem.
    pub async fn clear(&self) {
        self.write(|c| {
            c.execute_batch("DELETE FROM turns; DELETE FROM turn_records;")?;
            c.execute_batch("VACUUM;")
        })
        .await;
    }
}

/// Stream a legacy `benchmarks.json` into the database, one record at a time
/// (batched transactions), then rename it to `benchmarks.json.migrated`.
/// Re-running after an interrupted migration is safe: rows upsert by id.
fn migrate_legacy(data_dir: &Path, conn: &mut Connection) {
    use serde::de::{Deserializer, SeqAccess, Visitor};

    let legacy = data_dir.join(LEGACY_FILE);
    let Ok(file) = std::fs::File::open(&legacy) else { return };
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    tracing::info!(
        "migrating {} ({} MB) into {} — one-time, this can take a minute",
        legacy.display(),
        size / 1_000_000,
        DB_FILE
    );
    let started = std::time::Instant::now();

    struct Each<'a> {
        conn: &'a mut Connection,
    }
    impl<'de, 'a> Visitor<'de> for Each<'a> {
        type Value = usize;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an array of benchmark records")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<usize, A::Error> {
            use serde::de::Error;
            let mut n = 0usize;
            loop {
                let tx = self.conn.transaction().map_err(A::Error::custom)?;
                let mut in_batch = 0usize;
                let mut done = false;
                while in_batch < MIGRATE_BATCH {
                    match seq.next_element::<Benchmark>()? {
                        Some(b) => {
                            upsert(&tx, &b).map_err(A::Error::custom)?;
                            in_batch += 1;
                        }
                        None => {
                            done = true;
                            break;
                        }
                    }
                }
                tx.commit().map_err(A::Error::custom)?;
                n += in_batch;
                if done {
                    return Ok(n);
                }
            }
        }
    }

    let reader = std::io::BufReader::with_capacity(8 << 20, file);
    let mut de = serde_json::Deserializer::from_reader(reader);
    match (&mut de).deserialize_seq(Each { conn }) {
        Ok(n) => {
            let done = data_dir.join(LEGACY_DONE);
            match std::fs::rename(&legacy, &done) {
                Ok(()) => tracing::info!(
                    "migrated {n} turn(s) in {:.1?}; old file kept as {}",
                    started.elapsed(),
                    done.display()
                ),
                Err(e) => tracing::error!("migrated {n} turn(s) but could not rename {}: {e}", legacy.display()),
            }
        }
        Err(e) => tracing::error!(
            "migration of {} stopped: {e} — the file is left in place and will be retried at next boot",
            legacy.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bench(id: &str, session: &str, created: &str) -> Benchmark {
        serde_json::from_value(serde_json::json!({
            "id": id, "created_at": created, "kind": "chat", "label": "l",
            "provider": "p", "model": "m", "session": session,
            "prompt": "hello", "output": "world",
            "stats": { "total_ms": 10.0, "decode_ms": 5.0, "completion_tokens": 3,
                       "token_events": [{ "t_ms": 1.0, "chars": 1, "est_tokens": 1.0, "kind": "content", "text": "w" }] },
        }))
        .unwrap()
    }

    #[test]
    fn roundtrip_order_replace_delete_and_migration() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let dir = std::env::temp_dir().join(format!("velobench-ts-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            // legacy file: two sessions, out-of-order timestamps
            let legacy = vec![
                bench("a", "s1", "2026-01-02T00:00:00Z"),
                bench("b", "s1", "2026-01-01T00:00:00Z"),
                bench("c", "s2", "2026-01-03T00:00:00Z"),
            ];
            std::fs::write(dir.join(LEGACY_FILE), serde_json::to_vec(&legacy).unwrap()).unwrap();

            let st = TurnStore::open(dir.clone()).await.unwrap();
            assert!(!dir.join(LEGACY_FILE).exists(), "legacy file renamed");
            assert!(dir.join(LEGACY_DONE).exists(), "legacy file kept as backup");

            let sums = st.summaries().await;
            assert_eq!(sums.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["a", "b", "c"]);
            assert!(sums[0].unstamped, "unlabelled events need stamping");
            let s1 = st.session_turns("s1").await;
            assert_eq!(s1.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(), ["b", "a"], "oldest first");
            assert_eq!(st.session_kind("s2").await.as_deref(), Some("chat"));

            // replace keeps the insertion position; unknown id is refused
            let mut a = st.get("a").await.unwrap();
            a.output = "changed".into();
            assert!(st.replace(a).await);
            assert!(!st.replace(bench("zz", "s9", "2026-01-01T00:00:00Z")).await);
            let sums = st.summaries().await;
            assert_eq!(sums[0].id, "a");
            assert_eq!(sums[0].output, "changed");

            assert!(st.put(bench("d", "s2", "2026-01-04T00:00:00Z")).await);
            assert_eq!(st.raw_records_after(0, 10).await.len(), 4);
            assert!(st.delete("c").await);
            assert!(!st.delete("c").await);
            assert!(st.has_session("s2").await);
            assert!(st.delete("d").await);
            assert!(!st.has_session("s2").await);

            st.clear().await;
            assert!(st.summaries().await.is_empty());
            drop(st);
            let _ = std::fs::remove_dir_all(&dir);
        });
    }
}
