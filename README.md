# Inventario Android (Tauri 2 + Rust)

App de inventario para Android hecha con **Tauri 2**: la lógica y la base de datos están en Rust (SQLite local) y la interfaz en **Svelte 5**.

Funciones (v0.2):
- Login con usuario y contraseña (hash **argon2**, guardado en SQLite) y cambio de contraseña.
- CRUD de productos: SKU, nombre, descripción, moneda, precio, costo (con margen) y stock.
- **Multimoneda**: moneda base, monedas activas (19 monedas de Latinoamérica precargadas),
  precio de cada producto en su propia moneda y equivalentes en las demás.
- **Tasas de cambio** con historial y varios tipos (ej. BCV y Paralelo); aviso si la tasa no es de hoy.
- **Actualizaciones dentro de la app** desde GitHub Releases (al abrir y con un botón).
- Funciona sin conexión; los datos se guardan en el almacenamiento privado de la app.
- Montos exactos: enteros en la unidad mínima de cada moneda; tasas con `rust_decimal`.

> Usuario inicial: **admin** / **admin123**. Se crea solo la primera vez que se abre la app.

## Estructura

```
app/
├── src/                     # Frontend Svelte
│   ├── App.svelte           # Decide si mostrar Login o Productos
│   └── lib/
│       ├── api.js           # Llamadas a los comandos Rust (invoke)
│       ├── Login.svelte
│       ├── Products.svelte
│       └── ProductForm.svelte
└── src-tauri/               # Backend Rust
    ├── src/
    │   ├── lib.rs           # Comandos Tauri + estado (BD y sesión)
    │   ├── db.rs            # Migraciones SQLite y admin inicial
    │   ├── auth.rs          # Login y cambio de contraseña (argon2)
    │   ├── products.rs      # CRUD y validaciones de productos
    │   ├── currency.rs      # Monedas, tipos de tasa y tasas
    │   ├── money.rs         # Montos exactos y conversión
    │   ├── updater.rs       # Consulta latest.json y descarga la APK (verifica SHA-256)
    │   └── tests.rs         # Pruebas
    ├── plugins/tauri-plugin-apk-installer/  # Kotlin: abre el instalador de Android
    └── tauri.conf.json
```

## Requisitos

- Rust (stable) y Node.js 20 o superior.
- **Android:** Android Studio con SDK, **NDK** y JDK 17. Variables de entorno:
  ```bash
  export JAVA_HOME="/ruta/a/android-studio/jbr"
  export ANDROID_HOME="$HOME/Android/Sdk"
  export NDK_HOME="$ANDROID_HOME/ndk/<version>"
  ```
- Targets de Rust para Android:
  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
  ```
- Guía oficial: https://v2.tauri.app/start/prerequisites/

## Uso

```bash
cd app
npm install

# Escritorio (lo más rápido para desarrollar)
npm run tauri dev

# Android: solo la primera vez, genera el proyecto en src-tauri/gen/android
npm run tauri android init

# Android en un emulador o teléfono conectado por USB (con depuración USB activada)
npm run tauri android dev

# APK o AAB de release
npm run tauri android build
```

## Publicar versiones y actualizaciones

El workflow `.github/workflows/android-release.yml` compila la APK, la firma y la publica en **Releases**.

| Cómo se dispara | Resultado | ¿La ofrece el actualizador? |
|---|---|---|
| Tag `vX.Y.Z` | Release estable `Inventario X.Y.Z` + `latest.json` | **Sí** |
| Push con cambios en `app/` o *Run workflow* | Pre-release `build-N` (versión `X.Y.Z-build.N`) | No (solo pruebas) |

**Publicar una versión estable:**
1. Sube `version` en `app/src-tauri/tauri.conf.json` (ej. `0.3.0`) y haz push.
2. En GitHub: *Releases → Draft a new release → Choose a tag →* `v0.3.0` (crear) *→* escribe las notas *→ Publish*.
   El workflow compila y adjunta la APK y `latest.json`; tus notas se muestran en la app.

La app consulta `releases/latest/download/latest.json` al abrirse y desde *Ajustes → Buscar actualizaciones*;
descarga la APK, verifica el SHA-256 y abre el instalador de Android. Los datos se conservan.

**Actualización obligatoria:** en `app/actualizaciones.json` pon `"version_minima": "X.Y.Z"`; quien tenga una
versión menor no podrá cerrar el aviso hasta actualizar.

**Reglas:** cada versión estable debe ser mayor que la anterior (Android rechaza bajar de versión) y todas deben
firmarse con la misma llave (el workflow se niega a publicar una estable sin ella).

### Firma

Las versiones estables usan la llave permanente guardada en los secretos de GitHub. Para crear una nueva (solo si no tienes una):

```bash
keytool -genkeypair -keystore release.jks -alias inventario -keyalg RSA -keysize 2048 -validity 10000
base64 -w0 release.jks   # copia el resultado
```

y en GitHub → *Settings → Secrets and variables → Actions* agrega:

| Secreto | Valor |
|---|---|
| `ANDROID_KEYSTORE_BASE64` | salida de `base64 -w0 release.jks` |
| `ANDROID_KEYSTORE_PASSWORD` | contraseña del keystore |
| `ANDROID_KEY_ALIAS` | `inventario` |
| `ANDROID_KEY_PASSWORD` | contraseña de la llave (normalmente la misma) |

Guarda `release.jks` en un lugar seguro y **nunca lo subas al repositorio**: si lo pierdes, no podrás publicar actualizaciones con la misma firma.

## Pruebas

```bash
cd app/src-tauri
cargo test
```

## Próximos pasos

- Escaneo de códigos de barras (`tauri-plugin-barcode-scanner`).
- Obtener la tasa automáticamente (BCV u otras fuentes), manteniendo la carga manual.
- Movimientos de stock (entradas y salidas) en lugar de editar el stock directamente.
- Gestión de usuarios y roles.
- API en Axum + PostgreSQL y sincronización.
