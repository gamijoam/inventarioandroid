mod auth;
mod currency;
mod db;
mod error;
mod money;
mod products;
mod updater;

#[cfg(test)]
mod tests;

use std::sync::Mutex;

use rusqlite::Connection;
use serde::Serialize;
use tauri::{ipc::Channel, AppHandle, Manager, State};
use tauri_plugin_apk_installer::{ApkInstallerExt, InstallStatus};

use auth::User;
use currency::{Config, Tasa, TasaVigente, TipoTasa};
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

    /// Verifica la sesión y devuelve la conexión.
    fn auth_conn(&self) -> AppResult<(User, std::sync::MutexGuard<'_, Connection>)> {
        let user = self.current_user()?;
        Ok((user, self.conn()?))
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
    let (user, conn) = state.auth_conn()?;
    auth::change_password(&conn, &user.id, &current, &new)
}

// ---------- Productos ----------

#[tauri::command]
fn list_products(state: State<AppState>, search: Option<String>) -> AppResult<Vec<Product>> {
    let (_, conn) = state.auth_conn()?;
    products::list(&conn, search.as_deref())
}

#[tauri::command]
fn create_product(state: State<AppState>, input: ProductInput) -> AppResult<Product> {
    let (_, conn) = state.auth_conn()?;
    products::create(&conn, input)
}

#[tauri::command]
fn update_product(state: State<AppState>, id: String, input: ProductInput) -> AppResult<Product> {
    let (_, conn) = state.auth_conn()?;
    products::update(&conn, &id, input)
}

#[tauri::command]
fn delete_product(state: State<AppState>, id: String) -> AppResult<()> {
    let (_, conn) = state.auth_conn()?;
    products::delete(&conn, &id)
}

// ---------- Monedas y tasas ----------

#[tauri::command]
fn get_config(state: State<AppState>) -> AppResult<Config> {
    let (_, conn) = state.auth_conn()?;
    currency::config(&conn)
}

#[tauri::command]
fn set_moneda_base(state: State<AppState>, codigo: String) -> AppResult<Config> {
    let (_, conn) = state.auth_conn()?;
    currency::set_moneda_base(&conn, &codigo)?;
    currency::config(&conn)
}

#[tauri::command]
fn set_moneda_activa(state: State<AppState>, codigo: String, activa: bool) -> AppResult<Config> {
    let (_, conn) = state.auth_conn()?;
    currency::set_moneda_activa(&conn, &codigo, activa)?;
    currency::config(&conn)
}

#[tauri::command]
fn crear_tipo_tasa(state: State<AppState>, nombre: String) -> AppResult<TipoTasa> {
    let (_, conn) = state.auth_conn()?;
    currency::crear_tipo_tasa(&conn, &nombre)
}

#[tauri::command]
fn renombrar_tipo_tasa(state: State<AppState>, id: i64, nombre: String) -> AppResult<()> {
    let (_, conn) = state.auth_conn()?;
    currency::renombrar_tipo_tasa(&conn, id, &nombre)
}

#[tauri::command]
fn set_tipo_tasa_defecto(state: State<AppState>, id: i64) -> AppResult<()> {
    let (_, conn) = state.auth_conn()?;
    currency::set_tipo_tasa_defecto(&conn, id)
}

#[tauri::command]
fn eliminar_tipo_tasa(state: State<AppState>, id: i64) -> AppResult<()> {
    let (_, conn) = state.auth_conn()?;
    currency::eliminar_tipo_tasa(&conn, id)
}

#[tauri::command]
fn tasas_vigentes(state: State<AppState>) -> AppResult<Vec<TasaVigente>> {
    let (_, conn) = state.auth_conn()?;
    currency::tasas_vigentes(&conn)
}

#[tauri::command]
fn registrar_tasa(
    state: State<AppState>,
    moneda: String,
    tipo_id: i64,
    valor: String,
) -> AppResult<Tasa> {
    let (user, conn) = state.auth_conn()?;
    currency::registrar_tasa(&conn, &moneda, tipo_id, &valor, &user.id)
}

#[tauri::command]
fn historial_tasas(state: State<AppState>, moneda: String, tipo_id: i64) -> AppResult<Vec<Tasa>> {
    let (_, conn) = state.auth_conn()?;
    currency::historial(&conn, &moneda, tipo_id, 30)
}

// ---------- App y actualizaciones (no requieren sesión) ----------

#[derive(Serialize)]
struct AppInfo {
    version: String,
    esquema_bd: i64,
    plataforma: &'static str,
    repo: &'static str,
}

#[tauri::command]
fn app_info(app: AppHandle, state: State<AppState>) -> AppResult<AppInfo> {
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        esquema_bd: db::schema_version(&*state.conn()?)?,
        plataforma: std::env::consts::OS,
        repo: updater::REPO,
    })
}

#[tauri::command]
async fn check_update(app: AppHandle) -> AppResult<updater::UpdateInfo> {
    let actual = app.package_info().version.to_string();
    updater::consultar(&actual, app.apk_installer().supported()).await
}

/// Descarga la APK (con progreso), verifica el hash y abre el instalador de Android.
/// Devuelve "instalando" o "permiso" (hay que permitir instalar apps y volver a tocar).
#[tauri::command]
async fn install_update(
    app: AppHandle,
    url: String,
    sha256: String,
    on_progress: Channel<updater::Progreso>,
) -> AppResult<InstallStatus> {
    if !app.apk_installer().supported() {
        return Err(AppError::Update(
            "La instalación automática solo está disponible en Android".into(),
        ));
    }
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::Internal(e.to_string()))?
        .join("updates");
    let apk = updater::descargar(&url, &sha256, &dir, |p| {
        let _ = on_progress.send(p);
    })
    .await?;
    app.apk_installer()
        .install(&apk)
        .map_err(|e| AppError::Update(e.to_string()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_apk_installer::init())
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
            delete_product,
            get_config,
            set_moneda_base,
            set_moneda_activa,
            crear_tipo_tasa,
            renombrar_tipo_tasa,
            set_tipo_tasa_defecto,
            eliminar_tipo_tasa,
            tasas_vigentes,
            registrar_tasa,
            historial_tasas,
            app_info,
            check_update,
            install_update
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación");
}
