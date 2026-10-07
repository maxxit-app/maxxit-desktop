//! Durable delivery is independent of the application database and provider credentials.
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

const MAX_BYTES: i64 = 20 * 1024 * 1024;
const MAX_AGE: i64 = 7 * 86400;

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub enabled: bool,
    pub native_crashes: bool,
    pub configured: bool,
    pub generation: i64,
    pub pending: u64,
    pub oldest_age_seconds: u64,
    pub dropped: u64,
    pub last_accepted_at: Option<i64>,
    pub last_event_id: Option<String>,
    pub delivery_state: String,
}

pub struct Queue {
    db: Mutex<Connection>,
    persistence_failures: AtomicU64,
}
#[derive(Clone)]
pub struct Pending {
    pub id: String,
    pub body: Vec<u8>,
    pub category: String,
    pub attempts: u32,
}
impl Queue {
    pub fn open(directory: &Path) -> rusqlite::Result<Self> {
        std::fs::create_dir_all(directory)
            .map_err(|_| rusqlite::Error::InvalidPath(directory.into()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if std::fs::symlink_metadata(directory).is_ok_and(|m| m.file_type().is_symlink()) {
                return Err(rusqlite::Error::InvalidPath(directory.into()));
            }
            std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| rusqlite::Error::InvalidPath(directory.into()))?;
        }
        let path = directory.join("diagnostics.sqlite");
        if std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(rusqlite::Error::InvalidPath(path));
        }
        let db = Connection::open(&path)?;
        db.busy_timeout(std::time::Duration::from_millis(500))?;
        db.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA secure_delete=ON; PRAGMA foreign_keys=ON;
          CREATE TABLE IF NOT EXISTS state (id INTEGER PRIMARY KEY CHECK(id=1), enabled INTEGER NOT NULL DEFAULT 0, native_crashes INTEGER NOT NULL DEFAULT 0, epoch INTEGER NOT NULL DEFAULT 0, dropped INTEGER NOT NULL DEFAULT 0, accepted_at INTEGER, event_id TEXT, delivery TEXT NOT NULL DEFAULT 'disabled');
          INSERT OR IGNORE INTO state(id) VALUES(1);
          CREATE TABLE IF NOT EXISTS outbox(id TEXT PRIMARY KEY, body BLOB NOT NULL, category TEXT NOT NULL, created INTEGER NOT NULL, due INTEGER NOT NULL, attempts INTEGER NOT NULL DEFAULT 0, priority INTEGER NOT NULL);
          CREATE TABLE IF NOT EXISTS limits(category TEXT PRIMARY KEY, until INTEGER NOT NULL);
          CREATE TABLE IF NOT EXISTS history(id INTEGER PRIMARY KEY, body TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS lifecycle(id INTEGER PRIMARY KEY CHECK(id=1), active INTEGER NOT NULL);
          INSERT OR IGNORE INTO lifecycle VALUES(1,0);")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
        }
        Ok(Self {
            db: Mutex::new(db),
            persistence_failures: AtomicU64::new(0),
        })
    }
    fn with_db<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T> {
        let mut db = self.db.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        f(&mut db)
    }
    pub fn begin_session(&self) -> rusqlite::Result<bool> {
        self.with_db(|db| {
            let tx = db.transaction()?;
            let active =
                tx.query_row("SELECT active FROM lifecycle WHERE id=1", [], |r| r.get(0))?;
            tx.execute("UPDATE lifecycle SET active=1 WHERE id=1", [])?;
            tx.commit()?;
            Ok(active)
        })
    }
    pub fn end_session(&self) -> rusqlite::Result<()> {
        self.with_db(|db| {
            db.execute("UPDATE lifecycle SET active=0 WHERE id=1", [])
                .map(|_| ())
        })
    }
    pub fn consent(&self) -> (bool, bool, i64) {
        self.with_db(|db| {
            db.query_row(
                "SELECT enabled,native_crashes,epoch FROM state WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
        })
        .unwrap_or((false, false, -1))
    }
    pub fn set_consent(&self, enabled: bool, native: bool) -> rusqlite::Result<()> {
        self.with_db(|db| {
            let tx = db.transaction()?;
            tx.execute("UPDATE state SET enabled=?1,native_crashes=?2,epoch=epoch+1,delivery=?3 WHERE id=1", params![enabled, enabled && native, if enabled { "queued" } else { "disabled" }])?;
            // Any consent change removes old work. Previously captured content cannot be replayed under new consent.
            tx.execute_batch("DELETE FROM outbox; DELETE FROM history; DELETE FROM limits; UPDATE state SET event_id=NULL, accepted_at=NULL WHERE id=1;")?;
            tx.commit()?;
            db.execute_batch("VACUUM")
        })
    }
    pub fn store(
        &self,
        id: &str,
        body: &[u8],
        category: &str,
        priority: bool,
        epoch: i64,
        now: i64,
    ) -> rusqlite::Result<bool> {
        self.with_db(|db| {
            let tx = db.transaction()?;
            let permitted: bool = tx.query_row("SELECT enabled=1 AND epoch=?1 FROM state WHERE id=1", [epoch], |r| r.get(0))?;
            if !permitted { return Ok(false); }
            if body.len() as i64 > MAX_BYTES {
                tx.execute("UPDATE state SET dropped=dropped+1,delivery='oversized' WHERE id=1", [])?;
                tx.commit()?;
                return Ok(false);
            }
            tx.execute("INSERT OR IGNORE INTO outbox(id,body,category,created,due,priority) VALUES(?1,?2,?3,?4,?4,?5)", params![id,body,category,now,priority])?;
            let expired = tx.execute("DELETE FROM outbox WHERE created < ?1", [now-MAX_AGE])?;
            let mut removed = expired as i64;
            loop {
                let size: i64 = tx.query_row("SELECT COALESCE(SUM(length(body)),0) FROM outbox", [], |r| r.get(0))?;
                let count: i64 = tx.query_row("SELECT COUNT(*) FROM outbox", [], |r| r.get(0))?;
                if size <= MAX_BYTES && count <= 2000 { break; }
                tx.execute("DELETE FROM outbox WHERE id=(SELECT id FROM outbox ORDER BY priority ASC,created ASC,rowid ASC LIMIT 1)", [])?;
                removed += 1;
            }
            tx.execute("UPDATE state SET dropped=dropped+?1 WHERE id=1", [removed])?;
            let retained: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM outbox WHERE id=?1)", [id], |r| r.get(0))?;
            tx.commit()?;
            Ok(retained)
        }).inspect_err(|_| { self.persistence_failures.fetch_add(1,Ordering::Relaxed); })
    }
    pub fn remember(&self, body: &str, epoch: i64) -> rusqlite::Result<()> {
        self.with_db(|db| {
            let tx = db.transaction()?;
            let enabled: bool = tx.query_row("SELECT enabled=1 AND epoch=?1 FROM state WHERE id=1", [epoch], |r| r.get(0))?;
            if enabled {
                tx.execute("INSERT INTO history(body) VALUES(?1)", [body])?;
                tx.execute("DELETE FROM history WHERE id NOT IN(SELECT id FROM history ORDER BY id DESC LIMIT 100)", [])?;
            }
            tx.commit()
        })
    }
    pub fn history(&self) -> Vec<serde_json::Value> {
        self.with_db(|db| {
            let mut query = db.prepare("SELECT body FROM history ORDER BY id DESC LIMIT 50")?;
            let rows = query.query_map([], |r| r.get::<_, String>(0))?;
            Ok(rows
                .filter_map(|r| r.ok().and_then(|s| serde_json::from_str(&s).ok()))
                .collect())
        })
        .unwrap_or_default()
    }
    pub fn next(&self, now: i64) -> rusqlite::Result<Option<Pending>> {
        self.with_db(|db| {
            let expired=db.execute("DELETE FROM outbox WHERE created < ?1", [now-MAX_AGE])?;
            db.execute("UPDATE state SET dropped=dropped+?1 WHERE id=1", [expired])?;
            db.query_row("SELECT id,body,category,attempts FROM outbox WHERE due<=?1 AND (SELECT enabled FROM state WHERE id=1)=1 AND NOT EXISTS(SELECT 1 FROM limits WHERE (category=outbox.category OR category='all' OR (outbox.category='attachment' AND category='error')) AND until>?1) ORDER BY priority DESC,created ASC LIMIT 1", [now], |r| Ok(Pending{id:r.get(0)?,body:r.get(1)?,category:r.get(2)?,attempts:r.get(3)?})).optional()
        })
    }
    pub fn accepted(&self, item: &Pending, now: i64) -> rusqlite::Result<()> {
        self.with_db(|db| {
            let tx = db.transaction()?;
            tx.execute("DELETE FROM outbox WHERE id=?1", [&item.id])?;
            tx.execute(
                "UPDATE state SET accepted_at=?1,event_id=COALESCE(?2,event_id),delivery='accepted' WHERE id=1",
                params![now, if item.category=="log" {None} else {Some(&item.id)}],
            )?;
            tx.commit()
        })
    }
    pub fn rejected(&self, item: &Pending) -> rusqlite::Result<()> {
        self.with_db(|db| {
            let tx = db.transaction()?;
            tx.execute("DELETE FROM outbox WHERE id=?1", [&item.id])?;
            tx.execute(
                "UPDATE state SET dropped=dropped+1,delivery='rejected' WHERE id=1",
                [],
            )?;
            tx.commit()
        })
    }
    pub fn retry(&self, item: &Pending, now: i64, state: &str) -> rusqlite::Result<()> {
        self.with_db(|db| {
            let delay = (5_i64 * 2_i64.pow(item.attempts.min(10))).min(3600);
            let jitter = i64::from(uuid::Uuid::new_v4().as_bytes()[0]) % 5;
            db.execute(
                "UPDATE outbox SET attempts=attempts+1,due=?2 WHERE id=?1",
                params![item.id, now + delay + jitter],
            )?;
            db.execute("UPDATE state SET delivery=?1 WHERE id=1", [state])?;
            Ok(())
        })
    }
    pub fn rate_limit(&self, category: &str, until: i64) -> rusqlite::Result<()> {
        self.with_db(|db| {
            db.execute("INSERT INTO limits(category,until) VALUES(?1,?2) ON CONFLICT(category) DO UPDATE SET until=MAX(until,excluded.until)",params![category,until])?;
            db.execute("UPDATE state SET delivery='rate_limited' WHERE id=1", [])?;
            Ok(())
        })
    }
    pub fn health(&self, configured: bool, now: i64) -> Health {
        self.with_db(|db| {
            let expired=db.execute("DELETE FROM outbox WHERE created < ?1", [now-MAX_AGE])?;
            db.execute("UPDATE state SET dropped=dropped+?1 WHERE id=1", [expired])?;
            let mut health=db.query_row("SELECT enabled,native_crashes,dropped,accepted_at,event_id,delivery FROM state WHERE id=1", [], |r| Ok(Health{enabled:r.get(0)?,native_crashes:r.get(1)?,dropped:r.get(2)?,last_accepted_at:r.get(3)?,last_event_id:r.get(4)?,delivery_state:r.get(5)?,configured,..Health::default()}))?;
            let (count,oldest):(u64,Option<i64>)=db.query_row("SELECT COUNT(*),MIN(created) FROM outbox", [], |r| Ok((r.get(0)?,r.get(1)?)))?;
            health.generation=db.query_row("SELECT epoch FROM state WHERE id=1", [], |r|r.get(0))?;
            health.dropped+=self.persistence_failures.load(Ordering::Relaxed);
            if self.persistence_failures.load(Ordering::Relaxed)>0 { health.delivery_state="persistence_failed".into(); }
            health.pending=count;
            health.oldest_age_seconds=oldest.map(|t| (now-t).max(0) as u64).unwrap_or(0);
            let limited:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM limits WHERE until>?1)", [now], |r|r.get(0))?;
            if limited && health.enabled { health.delivery_state="rate_limited".into(); }
            if !configured && health.enabled { health.delivery_state="not_configured".into(); }
            Ok(health)
        }).unwrap_or_else(|_| Health{delivery_state:"storage_unavailable".into(),..Health::default()})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (std::path::PathBuf, Queue) {
        let dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let queue = Queue::open(&dir).unwrap();
        (dir, queue)
    }
    #[test]
    fn overflow_prioritizes_fatal_reports_and_counts_discards() {
        let (dir, q) = fixture();
        q.set_consent(true, false).unwrap();
        let e = q.consent().2;
        let large = vec![b'x'; 11 * 1024 * 1024];
        q.store("fatal", &large, "error", true, e, 100).unwrap();
        assert!(!q.store("routine", &large, "log", false, e, 101).unwrap());
        assert_eq!(q.health(true, 101).dropped, 1);
        assert_eq!(q.next(101).unwrap().unwrap().id, "fatal");
        let oversized = vec![b'x'; 21 * 1024 * 1024];
        assert!(!q
            .store("oversized", &oversized, "attachment", true, e, 102)
            .unwrap());
        assert_eq!(q.health(true, 102).dropped, 2);
        drop(q);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn consent_is_atomic_and_withdrawal_removes_pending_content() {
        let (dir, q) = fixture();
        assert!(!q.store("a", b"safe", "error", false, 0, 100).unwrap());
        q.set_consent(true, false).unwrap();
        let epoch = q.consent().2;
        assert!(q.store("a", b"safe", "error", false, epoch, 100).unwrap());
        q.remember("{}", epoch).unwrap();
        q.set_consent(false, false).unwrap();
        assert!(!q
            .store("late", b"safe", "error", false, epoch, 101)
            .unwrap());
        q.set_consent(true, false).unwrap();
        assert_eq!(q.health(true, 102).pending, 0);
        assert!(q.history().is_empty());
        drop(q);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn pending_events_survive_restart_and_keep_their_identity() {
        let (dir, q) = fixture();
        q.set_consent(true, false).unwrap();
        q.store("a", b"safe", "error", false, q.consent().2, 100)
            .unwrap();
        let first = q.next(100).unwrap().unwrap();
        q.retry(&first, 100, "retrying").unwrap();
        drop(q);
        let q = Queue::open(&dir).unwrap();
        assert!(q.next(101).unwrap().is_none());
        let item = q.next(200).unwrap().unwrap();
        assert_eq!(item.id, "a");
        assert_eq!(item.attempts, 1);
        q.accepted(&item, 200).unwrap();
        assert!(q.next(201).unwrap().is_none());
        drop(q);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn log_rate_limits_do_not_block_errors_and_expiry_is_visible() {
        let (dir, q) = fixture();
        q.set_consent(true, false).unwrap();
        let e = q.consent().2;
        q.store("log", b"safe", "log", false, e, 100).unwrap();
        q.store("error", b"safe", "error", true, e, 100).unwrap();
        q.rate_limit("log", 200).unwrap();
        assert_eq!(q.next(101).unwrap().unwrap().id, "error");
        q.next(100 + MAX_AGE + 1).unwrap();
        assert_eq!(q.health(true, 100 + MAX_AGE + 1).dropped, 2);
        drop(q);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
