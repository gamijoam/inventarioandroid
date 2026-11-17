use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Error de base de datos: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Usuario o contraseña incorrectos")]
    InvalidCredentials,
    #[error("Debes iniciar sesión")]
    Unauthorized,
    #[error("{0}")]
    Validation(String),
    #[error("Producto no encontrado")]
    NotFound,
    #[error("Error interno: {0}")]
    Internal(String),
}

// Tauri envía los errores al frontend serializados; los mandamos como texto legible.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
