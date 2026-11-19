//! Append-only ledger. Each row's hash covers the previous hash, so a quiet edit breaks the chain.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::todo,
        clippy::unimplemented,
        clippy::dbg_macro,
    )
)]

use castellan_core::Origin;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Ledger error.
#[derive(Debug, Error)]
pub enum StoreError {
    /// SQLite rejected a statement.
    #[error("ledger database: {0}")]
    Db(#[from] rusqlite::Error),
    /// The hash chain does not match the stored rows.
    #[error("ledger hash chain broken at sequence {0}")]
    Chain(i64),
    /// JSON for a row could not be read.
    #[error("ledger row was not valid JSON")]
    Json,
}

/// One recorded action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Plan id.
    pub plan_id: String,
    /// Who asked.
    pub origin: Origin,
    /// Unix seconds.
    pub recorded_unix: u64,
    /// Human summary. No secrets.
    pub summary: String,
    /// False for temp deletion.
    pub reversible: bool,
    /// What happened.
    pub result: String,
}

/// Open ledger.
pub struct Ledger {
    conn: Connection,
}

impl Ledger {
    /// Create or open a ledger file and its table.
    pub fn open(path: &std::path::Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ledger (
                seq INTEGER PRIMARY KEY AUTOINCREMENT,
                payload TEXT NOT NULL,
                prev_hash TEXT NOT NULL,
                hash TEXT NOT NULL
            );",
        )?;
        Ok(Self { conn })
    }

    /// Append one entry.
    pub fn append(&self, entry: &LedgerEntry) -> Result<(), StoreError> {
        let payload = serde_json::to_string(entry).map_err(|_| StoreError::Json)?;
        let prev = self.tip_hash()?;
        let hash = chain_hash(&prev, &payload);
        self.conn.execute(
            "INSERT INTO ledger (payload, prev_hash, hash) VALUES (?1, ?2, ?3)",
            (&payload, &prev, &hash),
        )?;
        Ok(())
    }

    /// Walk the chain. `Ok(())` means every row matches.
    pub fn verify(&self) -> Result<(), StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT seq, payload, prev_hash, hash FROM ledger ORDER BY seq")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut expected_prev = "genesis".to_owned();
        for row in rows {
            let (seq, payload, prev, hash) = row?;
            if prev != expected_prev || chain_hash(&prev, &payload) != hash {
                return Err(StoreError::Chain(seq));
            }
            expected_prev = hash;
        }
        Ok(())
    }

    /// Rows, oldest first.
    pub fn list(&self) -> Result<Vec<LedgerEntry>, StoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT payload FROM ledger ORDER BY seq")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for row in rows {
            let payload = row?;
            let entry = serde_json::from_str(&payload).map_err(|_| StoreError::Json)?;
            out.push(entry);
        }
        Ok(out)
    }

    fn tip_hash(&self) -> Result<String, StoreError> {
        let hash = self
            .conn
            .query_row(
                "SELECT hash FROM ledger ORDER BY seq DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(hash.unwrap_or_else(|| "genesis".to_owned()))
    }
}

fn chain_hash(prev: &str, payload: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(prev.as_bytes());
    hasher.update(b"\n");
    hasher.update(payload.as_bytes());
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_then_verify_and_detect_tamper() {
        let file = std::env::temp_dir().join(format!(
            "castellan-ledger-{}-{}.sqlite",
            std::process::id(),
            "chain"
        ));
        let _ = std::fs::remove_file(&file);
        let ledger = Ledger::open(&file).expect("open");
        ledger
            .append(&LedgerEntry {
                plan_id: "abc".to_owned(),
                origin: Origin::Cli,
                recorded_unix: 10,
                summary: "swept user temp".to_owned(),
                reversible: false,
                result: "deleted 2 files".to_owned(),
            })
            .expect("append");
        ledger.verify().expect("verify");
        ledger
            .conn
            .execute(
                "UPDATE ledger SET payload = '{\"tampered\":true}' WHERE seq = 1",
                [],
            )
            .expect("tamper");
        assert!(ledger.verify().is_err());
        let _ = std::fs::remove_file(&file);
    }
}
