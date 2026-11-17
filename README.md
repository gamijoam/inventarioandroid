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
