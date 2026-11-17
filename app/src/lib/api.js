import { invoke } from "@tauri-apps/api/core";

// Envoltorio de los comandos Rust definidos en src-tauri/src/lib.rs
export const api = {
  login: (username, password) => invoke("login", { username, password }),
  logout: () => invoke("logout"),
  currentUser: () => invoke("current_user"),
  changePassword: (current, nueva) => invoke("change_password", { current, new: nueva }),
  listProducts: (search) => invoke("list_products", { search: search || null }),
  createProduct: (input) => invoke("create_product", { input }),
  updateProduct: (id, input) => invoke("update_product", { id, input }),
  deleteProduct: (id) => invoke("delete_product", { id }),
};
