<script>
  import { cfg, monedaInfo, monedasActivas } from "./config.svelte.js";
  import { toDecimalString } from "./money.js";

  let { product = null, onSave, onCancel } = $props();

  // El formulario copia el producto UNA vez al abrirse (es intencional).
  // svelte-ignore state_referenced_locally
  const monedaInicial = product?.moneda ?? cfg.moneda_base;
  const dec = monedaInfo(monedaInicial)?.decimales ?? 2;

  // Copia local para no modificar el producto original hasta guardar.
  // svelte-ignore state_referenced_locally
  let form = $state({
    sku: product?.sku ?? "",
    nombre: product?.nombre ?? "",
    descripcion: product?.descripcion ?? "",
    moneda: monedaInicial,
    precio: product ? toDecimalString(product.precio, dec).replace(".", ",") : "",
    costo: product ? toDecimalString(product.costo, dec).replace(".", ",") : "",
    stock: product?.stock ?? 0,
  });
  let error = $state("");
  let saving = $state(false);

  const num = (s) => Number(String(s).replace(",", ".")) || 0;
  let margen = $derived.by(() => {
    const p = num(form.precio), c = num(form.costo);
    return p > 0 && c > 0 ? Math.round(((p - c) / p) * 1000) / 10 : null;
  });

  async function submit(e) {
    e.preventDefault();
    error = "";
    saving = true;
    try {
      await onSave({ ...form, stock: Math.trunc(Number(form.stock)) });
    } catch (err) {
      error = String(err);
    } finally {
      saving = false;
    }
  }
</script>

<div class="backdrop">
  <form class="card sheet" onsubmit={submit}>
    <h2>{product ? "Editar producto" : "Nuevo producto"}</h2>

    <label>SKU / Código <input bind:value={form.sku} required /></label>
    <label>Nombre <input bind:value={form.nombre} required /></label>
    <label>Descripción <textarea bind:value={form.descripcion} rows="2"></textarea></label>

    <label>
      Moneda del precio
      <select bind:value={form.moneda}>
        {#each monedasActivas() as m}
          <option value={m.codigo}>{m.codigo} · {m.nombre}</option>
        {/each}
      </select>
    </label>
    <div class="row">
      <label>Precio <input inputmode="decimal" placeholder="0,00" bind:value={form.precio} required /></label>
      <label>Costo <input inputmode="decimal" placeholder="0,00" bind:value={form.costo} /></label>
    </div>
    {#if margen !== null}
      <p class="muted small margin">Margen: {String(margen).replace(".", ",")}%</p>
    {/if}
    <label>Stock <input type="number" inputmode="numeric" step="1" min="0" bind:value={form.stock} required /></label>

    {#if error}<p class="error">{error}</p>{/if}

    <div class="row actions">
      <button type="button" onclick={onCancel}>Cancelar</button>
      <button class="primary" disabled={saving}>{saving ? "Guardando…" : "Guardar"}</button>
    </div>
  </form>
</div>
