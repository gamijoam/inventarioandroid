use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::{
    currency::{self, TablaConversion},
    error::{AppError, AppResult},
    money,
};

/// Precio expresado en otra moneda activa (según la tasa por defecto vigente).
#[derive(Debug, Clone, Serialize)]
pub struct Equivalente {
    pub moneda: String,
    pub precio: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Product {
    pub id: String,
    pub sku: String,
    pub nombre: String,
    pub descripcion: String,
    pub moneda: String,
    /// Montos en la unidad mínima de `moneda` (ej. centavos).
    pub precio: i64,
    pub costo: i64,
    pub stock: i64,
    pub created_at: String,
    pub updated_at: String,
    pub equivalentes: Vec<Equivalente>,
}

/// Datos del formulario. Los montos llegan como texto ("12,50") para no perder precisión.
#[derive(Debug, Clone, Deserialize)]
pub struct ProductInput {
    pub sku: String,
    pub nombre: String,
    #[serde(default)]
    pub descripcion: String,
    /// Si no viene, se usa la moneda base.
    #[serde(default)]
    pub moneda: Option<String>,
    pub precio: String,
    #[serde(default)]
    pub costo: String,
    pub stock: i64,
}

struct Validado {
    sku: String,
    nombre: String,
    descripcion: String,
    moneda: String,
    precio: i64,
    costo: i64,
    stock: i64,
}

fn validar(conn: &Connection, input: ProductInput) -> AppResult<Validado> {
    let sku = input.sku.trim().to_string();
    let nombre = input.nombre.trim().to_string();
    if sku.is_empty() {
        return Err(AppError::Validation("El SKU es obligatorio".into()));
    }
    if nombre.is_empty() {
        return Err(AppError::Validation("El nombre es obligatorio".into()));
    }
    if input.stock < 0 {
        return Err(AppError::Validation("El stock no puede ser negativo".into()));
    }
    let codigo = match input.moneda.as_deref().map(str::trim) {
        Some(c) if !c.is_empty() => c.to_string(),
        _ => currency::moneda_base(conn)?,
    };
    let m = currency::moneda(conn, &codigo)?;
    if !m.activa {
        return Err(AppError::Validation(format!(
            "La moneda {} no está activa. Actívala en Ajustes.",
            m.codigo
        )));
    }
    Ok(Validado {
        precio: money::parse_amount(&input.precio, m.decimales, "Precio", &m.codigo)?,
        costo: money::parse_amount(&input.costo, m.decimales, "Costo", &m.codigo)?,
        sku,
        nombre,
        descripcion: input.descripcion.trim().to_string(),
        moneda: m.codigo,
        stock: input.stock,
    })
}

const COLUMNS: &str =
    "id, sku, nombre, descripcion, moneda, precio, costo, stock, created_at, updated_at";

fn from_row(r: &Row) -> rusqlite::Result<Product> {
    Ok(Product {
        id: r.get(0)?,
        sku: r.get(1)?,
        nombre: r.get(2)?,
        descripcion: r.get(3)?,
        moneda: r.get(4)?,
        precio: r.get(5)?,
        costo: r.get(6)?,
        stock: r.get(7)?,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
        equivalentes: Vec::new(),
    })
}

fn con_equivalentes(mut p: Product, tabla: &TablaConversion) -> Product {
    p.equivalentes = tabla
        .activas
        .iter()
        .filter(|m| m.codigo != p.moneda)
        .filter_map(|m| {
            tabla
                .convertir(p.precio, &p.moneda, &m.codigo)
                .map(|precio| Equivalente {
                    moneda: m.codigo.clone(),
                    precio,
                })
        })
        .collect();
    p
}

fn map_unique(e: rusqlite::Error) -> AppError {
    if let rusqlite::Error::SqliteFailure(err, _) = &e {
        if err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE {
            return AppError::Validation("Ya existe un producto con ese SKU".into());
        }
    }
    AppError::Db(e)
}

pub fn list(conn: &Connection, search: Option<&str>) -> AppResult<Vec<Product>> {
    let tabla = TablaConversion::cargar(conn)?;
    let pattern = format!("%{}%", search.unwrap_or("").trim());
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM productos
         WHERE nombre LIKE ?1 OR sku LIKE ?1
         ORDER BY nombre COLLATE NOCASE"
    ))?;
    let rows = stmt.query_map(params![pattern], from_row)?;
    rows.map(|r| Ok(con_equivalentes(r?, &tabla))).collect()
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Product> {
    let p = conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM productos WHERE id = ?1"),
            params![id],
            from_row,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound,
            e => AppError::Db(e),
        })?;
    Ok(con_equivalentes(p, &TablaConversion::cargar(conn)?))
}

pub fn create(conn: &Connection, input: ProductInput) -> AppResult<Product> {
    let p = validar(conn, input)?;
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO productos (id, sku, nombre, descripcion, moneda, precio, costo, stock)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, p.sku, p.nombre, p.descripcion, p.moneda, p.precio, p.costo, p.stock],
    )
    .map_err(map_unique)?;
    get(conn, &id)
}

pub fn update(conn: &Connection, id: &str, input: ProductInput) -> AppResult<Product> {
    let p = validar(conn, input)?;
    let changed = conn
        .execute(
            "UPDATE productos
             SET sku = ?2, nombre = ?3, descripcion = ?4, moneda = ?5, precio = ?6, costo = ?7,
                 stock = ?8, updated_at = datetime('now')
             WHERE id = ?1",
            params![id, p.sku, p.nombre, p.descripcion, p.moneda, p.precio, p.costo, p.stock],
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
