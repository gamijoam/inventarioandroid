pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("La instalación de actualizaciones solo está disponible en Android")]
    Unsupported,
    #[cfg(mobile)]
    #[error("{0}")]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
}
