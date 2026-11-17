//! Plugin mínimo que abre el instalador de paquetes de Android con una APK descargada.
//! En escritorio no hace nada (devuelve `Unsupported`).

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

mod error;
pub use error::{Error, Result};

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallRequest {
    path: String,
}

/// Resultado de pedir la instalación.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InstallStatus {
    /// Se abrió el instalador de Android; el usuario debe confirmar.
    Instalando,
    /// Falta el permiso "instalar apps desconocidas"; se abrió la pantalla de ajustes.
    Permiso,
}

#[cfg_attr(not(target_os = "android"), allow(dead_code))]
#[derive(Debug, Deserialize)]
struct InstallResponse {
    status: InstallStatus,
}

pub struct ApkInstaller<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: tauri::plugin::PluginHandle<R>,
    #[cfg(not(target_os = "android"))]
    _marker: std::marker::PhantomData<fn() -> R>,
}

impl<R: Runtime> ApkInstaller<R> {
    /// Indica si este dispositivo puede instalar actualizaciones (solo Android).
    pub fn supported(&self) -> bool {
        cfg!(target_os = "android")
    }

    #[cfg(target_os = "android")]
    pub fn install(&self, path: &std::path::Path) -> Result<InstallStatus> {
        let res: InstallResponse = self.handle.run_mobile_plugin(
            "install",
            InstallRequest {
                path: path.to_string_lossy().into_owned(),
            },
        )?;
        Ok(res.status)
    }

    #[cfg(not(target_os = "android"))]
    pub fn install(&self, _path: &std::path::Path) -> Result<InstallStatus> {
        Err(Error::Unsupported)
    }
}

pub trait ApkInstallerExt<R: Runtime> {
    fn apk_installer(&self) -> &ApkInstaller<R>;
}

impl<R: Runtime, T: Manager<R>> ApkInstallerExt<R> for T {
    fn apk_installer(&self) -> &ApkInstaller<R> {
        self.state::<ApkInstaller<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("apk-installer")
        .setup(|app, _api| {
            #[cfg(target_os = "android")]
            let installer = ApkInstaller {
                handle: _api.register_android_plugin("com.inventario.apkinstaller", "ApkInstallerPlugin")?,
            };
            #[cfg(not(target_os = "android"))]
            let installer = ApkInstaller::<R> {
                _marker: std::marker::PhantomData,
            };
            app.manage(installer);
            Ok(())
        })
        .build()
}
