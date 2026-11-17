<script>
  let { product = null, onSave, onCancel } = $props();

  // Copia local para no modificar el producto original hasta guardar.
  let form = $state({
    sku: product?.sku ?? "",
    nombre: product?.nombre ?? "",
    descripcion: product?.descripcion ?? "",
    precio: product?.precio ?? 0,
    stock: product?.stock ?? 0,
  });
  let error = $state("");
  let saving = $state(false);

  async function submit(e) {
    e.preventDefault();
    error = "";
    saving = true;
    try {
      await onSave({
        ...form,
        precio: Number(form.precio),
        stock: Math.trunc(Number(form.stock)),
      });
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
    <div class="row">
      <label>Precio <input type="number" inputmode="decimal" step="0.01" min="0" bind:value={form.precio} required /></label>
      <label>Stock <input type="number" inputmode="numeric" step="1" min="0" bind:value={form.stock} required /></label>
    </div>

    {#if error}<p class="error">{error}</p>{/if}

    <div class="row actions">
      <button type="button" onclick={onCancel}>Cancelar</button>
      <button class="primary" disabled={saving}>{saving ? "Guardando…" : "Guardar"}</button>
    </div>
  </form>
</div>
