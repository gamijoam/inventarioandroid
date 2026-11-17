mod auth;
mod db;
mod error;
mod products;

#[cfg(test)]
mod tests;

use std::sync::Mutex;

use rusqlite::Connection;
use tauri::{Manager, State};

use auth::User;
use error::{AppError, AppResult};
use products::{Product, ProductInput};

/// Estado global de la app: conexión SQLite y usuario con sesión iniciada.
pub struct AppState {
    db: Mutex<Connection>,
    session: Mutex<Option<User>>,
}

impl AppState {
    fn conn(&self) -> AppResult<std::sync::MutexGuard<'_, Connection>> {
        self.db
            .lock()
            .map_err(|_| AppError::Internal("base de datos bloqueada".into()))
    }

    fn current_user(&self) -> AppResult<User> {
        self.session
            .lock()
            .map_err(|_| AppError::Internal("sesión bloqueada".into()))?
            .clone()
            .ok_or(AppError::Unauthorized)
    }
}

// ---------- Autenticación ----------

#[tauri::command]
fn login(state: State<AppState>, username: String, password: String) -> AppResult<User> {
    let user = auth::login(&*state.conn()?, &username, &password)?;
    *state.session.lock().unwrap() = Some(user.clone());
    Ok(user)
}

#[tauri::command]
fn logout(state: State<AppState>) {
    *state.session.lock().unwrap() = None;
}

#[tauri::command]
fn current_user(state: State<AppState>) -> Option<User> {
    state.session.lock().unwrap().clone()
}

#[tauri::command]
fn change_password(state: State<AppState>, current: String, new: String) -> AppResult<()> {
    let user = state.current_user()?;
    auth::change_password(&*state.conn()?, &user.id, &current, &new)
}

// ---------- Productos (requieren sesión) ----------

#[tauri::command]
fn list_products(state: State<AppState>, search: Option<String>) -> AppResult<Vec<Product>> {
    state.current_user()?;
    products::list(&*state.conn()?, search.as_deref())
}

#[tauri::command]
fn create_product(state: State<AppState>, input: ProductInput) -> AppResult<Product> {
    state.current_user()?;
    products::create(&*state.conn()?, input)
}

#[tauri::command]
fn update_product(state: State<AppState>, id: String, input: ProductInput) -> AppResult<Product> {
    state.current_user()?;
    products::update(&*state.conn()?, &id, input)
}

#[tauri::command]
fn delete_product(state: State<AppState>, id: String) -> AppResult<()> {
    state.current_user()?;
    products::delete(&*state.conn()?, &id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // En Android esto apunta al almacenamiento privado de la app.
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("inventario.db"))?;
            app.manage(AppState {
                db: Mutex::new(conn),
                session: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login,
            logout,
            current_user,
            change_password,
            list_products,
            create_product,
            update_product,
            delete_product
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación");
}
