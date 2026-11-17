use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub nombre: String,
}

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(e.to_string()))
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

pub fn login(conn: &Connection, username: &str, password: &str) -> AppResult<User> {
    let row = conn
        .query_row(
            "SELECT id, username, nombre, password_hash FROM usuarios WHERE username = ?1",
            params![username.trim()],
            |r| {
                Ok((
                    User {
                        id: r.get(0)?,
                        username: r.get(1)?,
                        nombre: r.get(2)?,
                    },
                    r.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;

    match row {
        Some((user, hash)) if verify_password(password, &hash) => Ok(user),
        _ => Err(AppError::InvalidCredentials),
    }
}

pub fn change_password(conn: &Connection, user_id: &str, current: &str, new: &str) -> AppResult<()> {
    if new.chars().count() < 6 {
        return Err(AppError::Validation(
            "La nueva contraseña debe tener al menos 6 caracteres".into(),
        ));
    }
    let hash: String = conn.query_row(
        "SELECT password_hash FROM usuarios WHERE id = ?1",
        params![user_id],
        |r| r.get(0),
    )?;
    if !verify_password(current, &hash) {
        return Err(AppError::Validation("La contraseña actual no es correcta".into()));
    }
    conn.execute(
        "UPDATE usuarios SET password_hash = ?1 WHERE id = ?2",
        params![hash_password(new)?, user_id],
    )?;
    Ok(())
}
