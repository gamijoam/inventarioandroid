use std::path::Path;

use rusqlite::{params, Connection};

use crate::{auth, error::AppResult};

pub const DEFAULT_ADMIN_USER: &str = "admin";
pub const DEFAULT_ADMIN_PASSWORD: &str = "admin123";

/// Monedas precargadas: (código ISO, nombre, símbolo, decimales).
pub const MONEDAS: &[(&str, &str, &str, u32)] = &[
    ("USD", "Dólar estadounidense", "$", 2),
    ("EUR", "Euro", "€", 2),
    ("VES", "Bolívar venezolano", "Bs", 2),
    ("COP", "Peso colombiano", "$", 0),
    ("MXN", "Peso mexicano", "$", 2),
    ("ARS", "Peso argentino", "$", 2),
    ("CLP", "Peso chileno", "$", 0),
    ("PEN", "Sol peruano", "S/", 2),
    ("BOB", "Boliviano", "Bs", 2),
    ("BRL", "Real brasileño", "R$", 2),
    ("UYU", "Peso uruguayo", "$U", 2),
    ("PYG", "Guaraní paraguayo", "₲", 0),
    ("GTQ", "Quetzal guatemalteco", "Q", 2),
    ("HNL", "Lempira hondureño", "L", 2),
    ("NIO", "Córdoba nicaragüense", "C$", 2),
    ("CRC", "Colón costarricense", "₡", 2),
    ("PAB", "Balboa panameño", "B/.", 2),
    ("DOP", "Peso dominicano", "RD$", 2),
    ("CUP", "Peso cubano", "$", 2),
];

/// Cada migración se aplica una sola vez, en orden. `PRAGMA user_version` guarda
/// cuántas se aplicaron. NUNCA modifiques una migración ya publicada: agrega una nueva.
const MIGRATIONS: &[fn(&Connection) -> AppResult<()>] = &[m1_inicial, m2_multimoneda];

pub fn open(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    init(&conn)?;
    Ok(conn)
}

pub fn init(conn: &Connection) -> AppResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
    migrate(conn)?;
    seed_admin(conn)
}

fn migrate(conn: &Connection) -> AppResult<()> {
    let current: usize = conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as usize;
    for (i, m) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.unchecked_transaction()?;
        m(&tx)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

pub fn schema_version(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
}

/// v0.1.x: esquema original (IF NOT EXISTS para bases creadas antes de las migraciones).
fn m1_inicial(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
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
    Ok(())
}

/// v0.2.0: monedas, tasas, precios en unidad mínima y costo.
fn m2_multimoneda(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE monedas (
            codigo    TEXT PRIMARY KEY,
            nombre    TEXT NOT NULL,
            simbolo   TEXT NOT NULL,
            decimales INTEGER NOT NULL CHECK (decimales BETWEEN 0 AND 4),
            activa    INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE configuracion (
            clave TEXT PRIMARY KEY,
            valor TEXT NOT NULL
        );

        CREATE TABLE tipos_tasa (
            id         INTEGER PRIMARY KEY,
            nombre     TEXT NOT NULL UNIQUE COLLATE NOCASE,
            es_defecto INTEGER NOT NULL DEFAULT 0
        );

        -- valor = cuántas unidades de `moneda` equivalen a 1 unidad de `moneda_base`.
        CREATE TABLE tasas (
            id           TEXT PRIMARY KEY,
            moneda_base  TEXT NOT NULL REFERENCES monedas(codigo),
            moneda       TEXT NOT NULL REFERENCES monedas(codigo),
            tipo_id      INTEGER NOT NULL REFERENCES tipos_tasa(id) ON DELETE CASCADE,
            valor        TEXT NOT NULL,
            fecha        TEXT NOT NULL,
            origen       TEXT NOT NULL DEFAULT 'manual',
            usuario_id   TEXT REFERENCES usuarios(id),
            created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
        );
        CREATE INDEX idx_tasas_vigente ON tasas (moneda_base, moneda, tipo_id, fecha DESC, created_at DESC);

        CREATE TABLE productos_v2 (
            id          TEXT PRIMARY KEY,
            sku         TEXT NOT NULL UNIQUE COLLATE NOCASE,
            nombre      TEXT NOT NULL,
            descripcion TEXT NOT NULL DEFAULT '',
            moneda      TEXT NOT NULL REFERENCES monedas(codigo),
            precio      INTEGER NOT NULL DEFAULT 0 CHECK (precio >= 0),
            costo       INTEGER NOT NULL DEFAULT 0 CHECK (costo >= 0),
            stock       INTEGER NOT NULL DEFAULT 0 CHECK (stock >= 0),
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )?;

    for (codigo, nombre, simbolo, decimales) in MONEDAS {
        conn.execute(
            "INSERT INTO monedas (codigo, nombre, simbolo, decimales, activa) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![codigo, nombre, simbolo, decimales, *codigo == "USD"],
        )?;
    }
    conn.execute("INSERT INTO configuracion (clave, valor) VALUES ('moneda_base', 'USD')", [])?;
    conn.execute("INSERT INTO tipos_tasa (nombre, es_defecto) VALUES ('Oficial', 1)", [])?;

    // Los precios existentes estaban en dólares como número decimal -> centavos.
    conn.execute_batch(
        r#"
        INSERT INTO productos_v2 (id, sku, nombre, descripcion, moneda, precio, costo, stock, created_at, updated_at)
        SELECT id, sku, nombre, descripcion, 'USD', CAST(ROUND(precio * 100) AS INTEGER), 0, stock, created_at, updated_at
        FROM productos;
        DROP TABLE productos;
        ALTER TABLE productos_v2 RENAME TO productos;
        "#,
    )?;
    Ok(())
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
