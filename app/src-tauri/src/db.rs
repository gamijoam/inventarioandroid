use std::path::Path;

use rusqlite::{params, Connection};

use crate::{auth, error::AppResult};

pub const DEFAULT_ADMIN_USER: &str = "admin";
pub const DEFAULT_ADMIN_PASSWORD: &str = "admin123";

/// Abre (o crea) la base de datos, aplica migraciones y crea el admin inicial.
pub fn open(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    init(&conn)?;
    Ok(conn)
}

pub fn init(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;

        CREATE TABLE IF NOT EXISTS usuarios (
            id            TEXT PRIMARY KEY,
            username      TEXT NOT NULL UNIQUE COLLATE NOCASE,
            nombre        TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            created_at    TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS productos (
            id          TEXT PRIMARY KEY,
            sku         TEXT NOT NULL UNIQUE COLLATE NOCASE,
            nombre      TEXT NOT NULL,
            descripcion TEXT NOT NULL DEFAULT '',
            precio      REAL NOT NULL DEFAULT 0 CHECK (precio >= 0),
            stock       INTEGER NOT NULL DEFAULT 0 CHECK (stock >= 0),
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )?;
    seed_admin(conn)
}

fn seed_admin(conn: &Connection) -> AppResult<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM usuarios", [], |r| r.get(0))?;
    if count == 0 {
        let hash = auth::hash_password(DEFAULT_ADMIN_PASSWORD)?;
        conn.execute(
            "INSERT INTO usuarios (id, username, nombre, password_hash) VALUES (?1, ?2, ?3, ?4)",
            params![
                uuid::Uuid::now_v7().to_string(),
                DEFAULT_ADMIN_USER,
                "Administrador",
                hash
            ],
        )?;
    }
    Ok(())
}
