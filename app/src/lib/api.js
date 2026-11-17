import { invoke, Channel } from "@tauri-apps/api/core";

// Envoltorio de los comandos Rust definidos en src-tauri/src/lib.rs
export const api = {
  // Sesión
  login: (username, password) => invoke("login", { username, password }),
  logout: () => invoke("logout"),
  currentUser: () => invoke("current_user"),
  changePassword: (current, nueva) => invoke("change_password", { current, new: nueva }),

  // Productos
  listProducts: (search) => invoke("list_products", { search: search || null }),
  createProduct: (input) => invoke("create_product", { input }),
  updateProduct: (id, input) => invoke("update_product", { id, input }),
  deleteProduct: (id) => invoke("delete_product", { id }),

  // Monedas y tasas
  getConfig: () => invoke("get_config"),
  setMonedaBase: (codigo) => invoke("set_moneda_base", { codigo }),
  setMonedaActiva: (codigo, activa) => invoke("set_moneda_activa", { codigo, activa }),
  crearTipoTasa: (nombre) => invoke("crear_tipo_tasa", { nombre }),
  renombrarTipoTasa: (id, nombre) => invoke("renombrar_tipo_tasa", { id, nombre }),
  setTipoTasaDefecto: (id) => invoke("set_tipo_tasa_defecto", { id }),
  eliminarTipoTasa: (id) => invoke("eliminar_tipo_tasa", { id }),
  tasasVigentes: () => invoke("tasas_vigentes"),
  registrarTasa: (moneda, tipoId, valor) => invoke("registrar_tasa", { moneda, tipoId, valor }),
  historialTasas: (moneda, tipoId) => invoke("historial_tasas", { moneda, tipoId }),

  // App
  appInfo: () => invoke("app_info"),
  checkUpdate: () => invoke("check_update"),
  installUpdate: (url, sha256, onProgress) => {
    const ch = new Channel();
    ch.onmessage = onProgress;
    return invoke("install_update", { url, sha256, onProgress: ch });
  },
};
