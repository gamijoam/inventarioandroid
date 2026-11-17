// Formateo de montos guardados en unidad mínima (centavos, etc.) sin perder precisión.

/** 1250 con 2 decimales -> "12.50" (texto exacto, apto para un input). */
export function toDecimalString(minor, decimales) {
  const neg = minor < 0;
  let s = String(Math.abs(minor)).padStart(decimales + 1, "0");
  if (decimales > 0) s = s.slice(0, -decimales) + "." + s.slice(-decimales);
  return (neg ? "-" : "") + s;
}

/** Para mostrar: "Bs 1.234,56" / "$ 12,50" (formato latino: miles con punto, decimales con coma). */
export function formatMoney(minor, moneda) {
  if (!moneda) return String(minor);
  const [ent, dec] = toDecimalString(minor, moneda.decimales).split(".");
  const miles = ent.replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  return `${moneda.simbolo} ${dec ? `${miles},${dec}` : miles}`;
}

/** Texto de tasa "36.5" -> "36,5" */
export function formatRate(valor) {
  const [ent, dec] = String(valor).split(".");
  const miles = ent.replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  return dec ? `${miles},${dec}` : miles;
}

export function formatDate(iso) {
  if (!iso) return "";
  const [y, m, d] = iso.slice(0, 10).split("-");
  return `${d}/${m}/${y}`;
}
