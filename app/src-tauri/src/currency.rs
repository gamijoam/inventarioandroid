//! Configuración de monedas, tipos de tasa y tasas de cambio.

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
use rust_decimal::Decimal;
use serde::Serialize;

use crate::{
    error::{AppError, AppResult},
    money,
};

#[derive(Debug, Clone, Serialize)]
pub struct Moneda {
    pub codigo: String,
    pub nombre: String,
    pub simbolo: String,
    pub decimales: u32,
    pub activa: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TipoTasa {
    pub id: i64,
    pub nombre: String,
    pub es_defecto: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Config {
    pub moneda_base: String,
    pub monedas: Vec<Moneda>,
    pub tipos_tasa: Vec<TipoTasa>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tasa {
    pub id: String,
    pub moneda_base: String,
    pub moneda: String,
    pub tipo_id: i64,
    /// Texto decimal exacto, ej. "36.5"
    pub valor: String,
    pub fecha: String,
    pub origen: String,
}

/// Tasa vigente de una moneda para un tipo (la más reciente), o `None` si nunca se cargó.
#[derive(Debug, Clone, Serialize)]
pub struct TasaVigente {
    pub moneda: String,
    pub tipo_id: i64,
    pub tipo_nombre: String,
    pub es_defecto: bool,
    pub tasa: Option<Tasa>,
    /// true si no hay tasa o si es de un día anterior a hoy.
    pub desactualizada: bool,
}

pub fn hoy() -> String {
    chrono::Local::now().date_naive().format("%Y-%m-%d").to_string()
}

// ---------- Lectura ----------

pub fn moneda_base(conn: &Connection) -> AppResult<String> {
    Ok(conn.query_row(
        "SELECT valor FROM configuracion WHERE clave = 'moneda_base'",
        [],
        |r| r.get(0),
    )?)
}

pub fn monedas(conn: &Connection) -> AppResult<Vec<Moneda>> {
    let mut stmt = conn.prepare(
        "SELECT codigo, nombre, simbolo, decimales, activa FROM monedas ORDER BY activa DESC, codigo",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Moneda {
            codigo: r.get(0)?,
            nombre: r.get(1)?,
            simbolo: r.get(2)?,
            decimales: r.get(3)?,
            activa: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn moneda(conn: &Connection, codigo: &str) -> AppResult<Moneda> {
    monedas(conn)?
        .into_iter()
        .find(|m| m.codigo.eq_ignore_ascii_case(codigo))
        .ok_or_else(|| AppError::Validation(format!("La moneda {codigo} no existe")))
}

pub fn tipos_tasa(conn: &Connection) -> AppResult<Vec<TipoTasa>> {
    let mut stmt =
        conn.prepare("SELECT id, nombre, es_defecto FROM tipos_tasa ORDER BY es_defecto DESC, nombre")?;
    let rows = stmt.query_map([], |r| {
        Ok(TipoTasa {
            id: r.get(0)?,
            nombre: r.get(1)?,
            es_defecto: r.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn config(conn: &Connection) -> AppResult<Config> {
    Ok(Config {
        moneda_base: moneda_base(conn)?,
        monedas: monedas(conn)?,
        tipos_tasa: tipos_tasa(conn)?,
    })
}

fn tasa_from_row(r: &rusqlite::Row) -> rusqlite::Result<Tasa> {
    Ok(Tasa {
        id: r.get(0)?,
        moneda_base: r.get(1)?,
        moneda: r.get(2)?,
        tipo_id: r.get(3)?,
        valor: r.get(4)?,
        fecha: r.get(5)?,
        origen: r.get(6)?,
    })
}

const TASA_COLS: &str = "id, moneda_base, moneda, tipo_id, valor, fecha, origen";

fn ultima_tasa(conn: &Connection, base: &str, moneda: &str, tipo_id: i64) -> AppResult<Option<Tasa>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {TASA_COLS} FROM tasas
                 WHERE moneda_base = ?1 AND moneda = ?2 AND tipo_id = ?3
                 ORDER BY fecha DESC, created_at DESC LIMIT 1"
            ),
            params![base, moneda, tipo_id],
            tasa_from_row,
        )
        .optional()?)
}

/// Tasa vigente para cada moneda activa (distinta de la base) y cada tipo de tasa.
pub fn tasas_vigentes(conn: &Connection) -> AppResult<Vec<TasaVigente>> {
    let base = moneda_base(conn)?;
    let hoy = hoy();
    let tipos = tipos_tasa(conn)?;
    let mut out = Vec::new();
    for m in monedas(conn)?.into_iter().filter(|m| m.activa && m.codigo != base) {
        for t in &tipos {
            let tasa = ultima_tasa(conn, &base, &m.codigo, t.id)?;
            out.push(TasaVigente {
                moneda: m.codigo.clone(),
                tipo_id: t.id,
                tipo_nombre: t.nombre.clone(),
                es_defecto: t.es_defecto,
                desactualizada: tasa.as_ref().is_none_or(|x| x.fecha < hoy),
                tasa,
            });
        }
    }
    Ok(out)
}

pub fn historial(conn: &Connection, moneda: &str, tipo_id: i64, limite: u32) -> AppResult<Vec<Tasa>> {
    let base = moneda_base(conn)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {TASA_COLS} FROM tasas
         WHERE moneda_base = ?1 AND moneda = ?2 AND tipo_id = ?3
         ORDER BY fecha DESC, created_at DESC LIMIT ?4"
    ))?;
    let rows = stmt.query_map(params![base, moneda, tipo_id, limite], tasa_from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Tabla para convertir precios: moneda -> (decimales, tasa contra la base) usando el tipo por defecto.
/// Solo incluye monedas con tasa conocida (la base siempre, con tasa 1).
pub struct TablaConversion {
    pub activas: Vec<Moneda>,
    tasas: HashMap<String, (u32, Decimal)>,
}

impl TablaConversion {
    pub fn cargar(conn: &Connection) -> AppResult<Self> {
        let base = moneda_base(conn)?;
        let todas = monedas(conn)?;
        let defecto: Option<i64> = conn
            .query_row("SELECT id FROM tipos_tasa WHERE es_defecto = 1", [], |r| r.get(0))
            .optional()?;
        let mut tasas = HashMap::new();
        for m in &todas {
            if m.codigo == base {
                tasas.insert(m.codigo.clone(), (m.decimales, Decimal::ONE));
            } else if let Some(tipo) = defecto {
                if let Some(t) = ultima_tasa(conn, &base, &m.codigo, tipo)? {
                    if let Ok(v) = money::parse_decimal(&t.valor, "tasa") {
                        tasas.insert(m.codigo.clone(), (m.decimales, v));
                    }
                }
            }
        }
        Ok(Self {
            activas: todas.into_iter().filter(|m| m.activa).collect(),
            tasas,
        })
    }

    pub fn convertir(&self, minor: i64, desde: &str, hacia: &str) -> Option<i64> {
        let (d1, t1) = self.tasas.get(desde)?;
        let (d2, t2) = self.tasas.get(hacia)?;
        money::convert(minor, *d1, *t1, *d2, *t2)
    }
}

// ---------- Escritura ----------

pub fn set_moneda_base(conn: &Connection, codigo: &str) -> AppResult<()> {
    let m = moneda(conn, codigo)?;
    conn.execute("UPDATE monedas SET activa = 1 WHERE codigo = ?1", params![m.codigo])?;
    conn.execute(
        "UPDATE configuracion SET valor = ?1 WHERE clave = 'moneda_base'",
        params![m.codigo],
    )?;
    Ok(())
}

pub fn set_moneda_activa(conn: &Connection, codigo: &str, activa: bool) -> AppResult<()> {
    let m = moneda(conn, codigo)?;
    if !activa {
        if m.codigo == moneda_base(conn)? {
            return Err(AppError::Validation(
                "No puedes desactivar la moneda base. Cambia primero la moneda base.".into(),
            ));
        }
        let usados: i64 = conn.query_row(
            "SELECT COUNT(*) FROM productos WHERE moneda = ?1",
            params![m.codigo],
            |r| r.get(0),
        )?;
        if usados > 0 {
            return Err(AppError::Validation(format!(
                "Hay {usados} producto(s) con precio en {}. Cámbialos antes de desactivarla.",
                m.codigo
            )));
        }
    }
    conn.execute(
        "UPDATE monedas SET activa = ?2 WHERE codigo = ?1",
        params![m.codigo, activa],
    )?;
    Ok(())
}

pub fn crear_tipo_tasa(conn: &Connection, nombre: &str) -> AppResult<TipoTasa> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(AppError::Validation("El nombre del tipo de tasa es obligatorio".into()));
    }
    conn.execute("INSERT INTO tipos_tasa (nombre) VALUES (?1)", params![nombre])
        .map_err(|e| match e {
            rusqlite::Error::SqliteFailure(err, _)
                if err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
            {
                AppError::Validation(format!("Ya existe el tipo de tasa \"{nombre}\""))
            }
            e => AppError::Db(e),
        })?;
    Ok(TipoTasa {
        id: conn.last_insert_rowid(),
        nombre: nombre.into(),
        es_defecto: false,
    })
}

pub fn renombrar_tipo_tasa(conn: &Connection, id: i64, nombre: &str) -> AppResult<()> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(AppError::Validation("El nombre del tipo de tasa es obligatorio".into()));
    }
    let n = conn.execute("UPDATE tipos_tasa SET nombre = ?2 WHERE id = ?1", params![id, nombre])?;
    if n == 0 {
        return Err(AppError::Validation("El tipo de tasa no existe".into()));
    }
    Ok(())
}

pub fn set_tipo_tasa_defecto(conn: &Connection, id: i64) -> AppResult<()> {
    let existe: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM tipos_tasa WHERE id = ?1)",
        params![id],
        |r| r.get(0),
    )?;
    if !existe {
        return Err(AppError::Validation("El tipo de tasa no existe".into()));
    }
    conn.execute("UPDATE tipos_tasa SET es_defecto = (id = ?1)", params![id])?;
    Ok(())
}

pub fn eliminar_tipo_tasa(conn: &Connection, id: i64) -> AppResult<()> {
    let defecto: Option<bool> = conn
        .query_row("SELECT es_defecto FROM tipos_tasa WHERE id = ?1", params![id], |r| r.get(0))
        .optional()?;
    match defecto {
        None => Err(AppError::Validation("El tipo de tasa no existe".into())),
        Some(true) => Err(AppError::Validation(
            "No puedes eliminar el tipo de tasa por defecto".into(),
        )),
        Some(false) => {
            conn.execute("DELETE FROM tipos_tasa WHERE id = ?1", params![id])?;
            Ok(())
        }
    }
}

/// Registra la tasa de hoy: "1 <base> = valor <moneda>".
pub fn registrar_tasa(
    conn: &Connection,
    moneda_codigo: &str,
    tipo_id: i64,
    valor: &str,
    usuario_id: &str,
) -> AppResult<Tasa> {
    let base = moneda_base(conn)?;
    let m = moneda(conn, moneda_codigo)?;
    if m.codigo == base {
        return Err(AppError::Validation("La moneda base siempre vale 1".into()));
    }
    let v = money::parse_decimal(valor, "Tasa")?.normalize();
    if v <= Decimal::ZERO {
        return Err(AppError::Validation("La tasa debe ser mayor que 0".into()));
    }
    let existe: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM tipos_tasa WHERE id = ?1)",
        params![tipo_id],
        |r| r.get(0),
    )?;
    if !existe {
        return Err(AppError::Validation("El tipo de tasa no existe".into()));
    }
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO tasas (id, moneda_base, moneda, tipo_id, valor, fecha, origen, usuario_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'manual', ?7)",
        params![id, base, m.codigo, tipo_id, v.to_string(), hoy(), usuario_id],
    )?;
    Ok(conn.query_row(
        &format!("SELECT {TASA_COLS} FROM tasas WHERE id = ?1"),
        params![id],
        tasa_from_row,
    )?)
}
