//! Actualizaciones desde GitHub Releases.
//!
//! El workflow publica `latest.json` en cada Release estable (tags `v*`). GitHub sirve
//! siempre el del último Release no-prerelease en una URL fija.

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, AppResult};

pub const REPO: &str = "gamijoam/inventarioandroid";

fn latest_url() -> String {
    format!("https://github.com/{REPO}/releases/latest/download/latest.json")
}

/// Solo se descargan APKs publicadas en los Releases de este repositorio.
fn url_permitida(url: &str) -> bool {
    url.starts_with(&format!("https://github.com/{REPO}/releases/download/"))
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub notas: String,
    #[serde(default)]
    pub version_minima: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub actual: String,
    pub disponible: bool,
    pub obligatoria: bool,
    pub version: String,
    pub notas: String,
    pub url: String,
    pub sha256: String,
    /// false en escritorio: allí solo se informa.
    pub puede_instalar: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Progreso {
    pub descargado: u64,
    pub total: Option<u64>,
}

fn err(msg: impl Into<String>) -> AppError {
    AppError::Update(msg.into())
}

fn parse_version(v: &str) -> AppResult<semver::Version> {
    semver::Version::parse(v.trim().trim_start_matches('v'))
        .map_err(|_| err(format!("versión inválida: {v}")))
}

/// Compara la versión instalada con el manifiesto publicado.
pub fn evaluar(actual: &str, m: &Manifest, puede_instalar: bool) -> AppResult<UpdateInfo> {
    let instalada = parse_version(actual)?;
    let nueva = parse_version(&m.version)?;
    let disponible = nueva > instalada;
    let obligatoria = match &m.version_minima {
        Some(min) if disponible && !min.trim().is_empty() => instalada < parse_version(min)?,
        _ => false,
    };
    Ok(UpdateInfo {
        actual: actual.to_string(),
        disponible,
        obligatoria,
        version: nueva.to_string(),
        notas: m.notas.clone(),
        url: m.url.clone(),
        sha256: m.sha256.to_lowercase(),
        puede_instalar,
    })
}

/// Causa más profunda del error (ej. "certificado inválido"), útil para diagnosticar.
fn raiz(e: &(dyn std::error::Error + 'static)) -> String {
    let mut cur = e;
    while let Some(next) = cur.source() {
        cur = next;
    }
    cur.to_string()
}

fn client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("inventario-app")
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| err(e.to_string()))
}

pub async fn consultar(actual: &str, puede_instalar: bool) -> AppResult<UpdateInfo> {
    let resp = client()?
        .get(latest_url())
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| err(format!("Sin conexión con GitHub. Revisa tu internet. ({})", raiz(&e))))?;

    // Aún no hay ningún Release estable publicado.
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(UpdateInfo {
            actual: actual.to_string(),
            disponible: false,
            obligatoria: false,
            version: actual.to_string(),
            notas: String::new(),
            url: String::new(),
            sha256: String::new(),
            puede_instalar,
        });
    }
    if !resp.status().is_success() {
        return Err(err(format!("GitHub respondió {}", resp.status())));
    }
    let bytes = resp.bytes().await.map_err(|e| err(e.to_string()))?;
    let m: Manifest =
        serde_json::from_slice(&bytes).map_err(|_| err("latest.json tiene un formato inválido"))?;
    evaluar(actual, &m, puede_instalar)
}

async fn sha256_de(path: &Path) -> Option<String> {
    let data = tokio::fs::read(path).await.ok()?;
    Some(hex::encode(Sha256::digest(&data)))
}

/// Descarga la APK (o reutiliza una ya descargada con el mismo hash) y verifica el SHA-256.
pub async fn descargar(
    url: &str,
    sha256: &str,
    dir: &Path,
    on_progress: impl Fn(Progreso),
) -> AppResult<PathBuf> {
    if !url_permitida(url) {
        return Err(err("URL de descarga no permitida"));
    }
    let esperado = sha256.trim().to_lowercase();
    if esperado.len() != 64 {
        return Err(err("Hash SHA-256 inválido en latest.json"));
    }
    tokio::fs::create_dir_all(dir).await.map_err(|e| err(e.to_string()))?;
    let destino = dir.join("Inventario-update.apk");

    if sha256_de(&destino).await.as_deref() == Some(esperado.as_str()) {
        return Ok(destino);
    }

    let resp = client()?
        .get(url)
        .send()
        .await
        .map_err(|_| err("No se pudo iniciar la descarga"))?;
    if !resp.status().is_success() {
        return Err(err(format!("Descarga falló: {}", resp.status())));
    }
    let total = resp.content_length();
    let parcial = dir.join("Inventario-update.apk.part");
    let mut file = tokio::fs::File::create(&parcial)
        .await
        .map_err(|e| err(e.to_string()))?;
    let mut hasher = Sha256::new();
    let mut descargado = 0u64;
    let mut ultimo_aviso = 0u64;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| err("Se cortó la descarga. Inténtalo de nuevo."))?;
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(|e| err(e.to_string()))?;
        descargado += chunk.len() as u64;
        if descargado - ultimo_aviso >= 256 * 1024 {
            ultimo_aviso = descargado;
            on_progress(Progreso { descargado, total });
        }
    }
    file.flush().await.map_err(|e| err(e.to_string()))?;
    drop(file);
    on_progress(Progreso { descargado, total });

    if hex::encode(hasher.finalize()) != esperado {
        let _ = tokio::fs::remove_file(&parcial).await;
        return Err(err("La descarga está dañada (SHA-256 no coincide). Inténtalo de nuevo."));
    }
    tokio::fs::rename(&parcial, &destino)
        .await
        .map_err(|e| err(e.to_string()))?;
    Ok(destino)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str, minima: Option<&str>) -> Manifest {
        Manifest {
            version: version.into(),
            url: format!("https://github.com/{REPO}/releases/download/v{version}/x.apk"),
            sha256: "a".repeat(64),
            notas: "notas".into(),
            version_minima: minima.map(Into::into),
        }
    }

    #[test]
    fn detecta_version_nueva() {
        let i = evaluar("0.1.3", &manifest("0.2.0", None), true).unwrap();
        assert!(i.disponible && !i.obligatoria);
        assert!(!evaluar("0.2.0", &manifest("0.2.0", None), true).unwrap().disponible);
        assert!(!evaluar("0.3.0", &manifest("0.2.0", None), true).unwrap().disponible);
        // un build de prueba 0.2.0-build.5 es menor que la estable 0.2.0
        assert!(evaluar("0.2.0-build.5", &manifest("0.2.0", None), true).unwrap().disponible);
    }

    #[test]
    fn actualizacion_obligatoria() {
        assert!(evaluar("0.1.0", &manifest("0.3.0", Some("0.2.0")), true).unwrap().obligatoria);
        assert!(!evaluar("0.2.1", &manifest("0.3.0", Some("0.2.0")), true).unwrap().obligatoria);
        assert!(!evaluar("0.1.0", &manifest("0.3.0", Some("")), true).unwrap().obligatoria);
    }

    #[test]
    fn solo_urls_del_repo() {
        assert!(url_permitida(&format!("https://github.com/{REPO}/releases/download/v1/a.apk")));
        assert!(!url_permitida("https://evil.com/a.apk"));
        assert!(!url_permitida("https://github.com/otro/repo/releases/download/v1/a.apk"));
    }
}
