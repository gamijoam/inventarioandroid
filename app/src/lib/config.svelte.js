import { api } from "./api.js";

// Configuración de monedas compartida por todas las pantallas.
export const cfg = $state({ moneda_base: "USD", monedas: [], tipos_tasa: [], cargada: false });

export async function loadConfig() {
  Object.assign(cfg, await api.getConfig(), { cargada: true });
}

export function setConfig(c) {
  Object.assign(cfg, c, { cargada: true });
}

export function monedaInfo(codigo) {
  return cfg.monedas.find((m) => m.codigo === codigo);
}

export function monedasActivas() {
  return cfg.monedas.filter((m) => m.activa);
}
