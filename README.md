# Inventario Android (Tauri 2 + Rust)

App de inventario para Android hecha con **Tauri 2**: la lógica y la base de datos están en Rust (SQLite local) y la interfaz en **Svelte 5**.

Versión actual (MVP):
- Login con usuario y contraseña (hash **argon2**, guardado en SQLite).
- CRUD de productos: SKU, nombre, descripción, precio y stock, con búsqueda.
- Funciona sin conexión; los datos se guardan en el almacenamiento privado de la app.

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
    │   └── tests.rs         # Pruebas
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

## Descargar la APK (GitHub Actions)

El workflow `.github/workflows/android-release.yml` compila la APK, la firma y la publica en **Releases**.

- **Publicar una versión:** crea y sube un tag. La versión de la app se toma del tag.
  ```bash
  git tag v0.1.1
  git push origin v0.1.1
  ```
- **Compilar a mano:** pestaña *Actions* → *Android APK* → *Run workflow*. Crea un pre-release `build-N`.

Desde el teléfono, abre el Release, descarga `Inventario-X.Y.Z.apk` e instálala (Android pedirá permitir apps de origen desconocido).

### Firma (recomendado)

Sin configurar nada, cada build se firma con una llave temporal, así que para actualizar hay que desinstalar la versión anterior (y se pierden los datos). Para que las actualizaciones se instalen encima, crea **una sola vez** una llave:

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
- Movimientos de stock (entradas y salidas) en lugar de editar el stock directamente.
- Gestión de usuarios y roles.
- API en Axum + PostgreSQL y sincronización.
