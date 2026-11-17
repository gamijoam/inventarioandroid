use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
pub struct Product {
    pub id: String,
    pub sku: String,
    pub nombre: String,
    pub descripcion: String,
    pub precio: f64,
    pub stock: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Datos que envía el formulario para crear o editar.
#[derive(Debug, Clone, Deserialize)]
pub struct ProductInput {
    pub sku: String,
    pub nombre: String,
    #[serde(default)]
    pub descripcion: String,
    pub precio: f64,
    pub stock: i64,
}

impl ProductInput {
    fn validated(mut self) -> AppResult<Self> {
        self.sku = self.sku.trim().to_string();
        self.nombre = self.nombre.trim().to_string();
        self.descripcion = self.descripcion.trim().to_string();
        if self.sku.is_empty() {
            return Err(AppError::Validation("El SKU es obligatorio".into()));
        }
        if self.nombre.is_empty() {
            return Err(AppError::Validation("El nombre es obligatorio".into()));
        }
        if !self.precio.is_finite() || self.precio < 0.0 {
            return Err(AppError::Validation("El precio debe ser un número ≥ 0".into()));
        }
        if self.stock < 0 {
            return Err(AppError::Validation("El stock no puede ser negativo".into()));
        }
        Ok(self)
    }
}

const COLUMNS: &str = "id, sku, nombre, descripcion, precio, stock, created_at, updated_at";

fn from_row(r: &Row) -> rusqlite::Result<Product> {
    Ok(Product {
        id: r.get(0)?,
        sku: r.get(1)?,
        nombre: r.get(2)?,
        descripcion: r.get(3)?,
        precio: r.get(4)?,
        stock: r.get(5)?,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
    })
}

/// Convierte el error de SKU duplicado en un mensaje amigable.
fn map_unique(e: rusqlite::Error) -> AppError {
    if let rusqlite::Error::SqliteFailure(err, _) = &e {
        if err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE {
            return AppError::Validation("Ya existe un producto con ese SKU".into());
        }
    }
    AppError::Db(e)
}

pub fn list(conn: &Connection, search: Option<&str>) -> AppResult<Vec<Product>> {
    let pattern = format!("%{}%", search.unwrap_or("").trim());
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM productos
         WHERE nombre LIKE ?1 OR sku LIKE ?1
         ORDER BY nombre COLLATE NOCASE"
    ))?;
    let rows = stmt.query_map(params![pattern], from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Product> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM productos WHERE id = ?1"),
        params![id],
        from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
        e => AppError::Db(e),
    })
}

pub fn create(conn: &Connection, input: ProductInput) -> AppResult<Product> {
    let p = input.validated()?;
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO productos (id, sku, nombre, descripcion, precio, stock)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, p.sku, p.nombre, p.descripcion, p.precio, p.stock],
    )
    .map_err(map_unique)?;
    get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, input: ProductInput) -> AppResult<Product> {
    let p = input.validated()?;
    let changed = conn
        .execute(
            "UPDATE productos
             SET sku = ?2, nombre = ?3, descripcion = ?4, precio = ?5, stock = ?6,
                 updated_at = datetime('now')
             WHERE id = ?1",
            params![id, p.sku, p.nombre, p.descripcion, p.precio, p.stock],
        )
        .map_err(map_unique)?;
    if changed == 0 {
        return Err(AppError::NotFound);
    }
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    if conn.execute("DELETE FROM productos WHERE id = ?1", params![id])? == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}
