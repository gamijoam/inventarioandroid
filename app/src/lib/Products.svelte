<script>
  import { onMount } from "svelte";
  import { api } from "./api.js";
  import { cfg, monedaInfo } from "./config.svelte.js";
  import { formatMoney } from "./money.js";
  import ProductForm from "./ProductForm.svelte";

  let { user, onSesionExpirada, irATasas } = $props();

  let products = $state([]);
  let search = $state("");
  let error = $state("");
  let editing = $state(null); // null = cerrado, {} = nuevo, producto = editar
  let confirmId = $state(null); // producto pendiente de confirmar eliminación
  let tasasViejas = $state([]); // tasas por defecto sin cargar hoy

  async function load() {
    try {
      error = "";
      products = await api.listProducts(search);
      const vig = await api.tasasVigentes();
      tasasViejas = vig.filter((t) => t.es_defecto && t.desactualizada);
    } catch (err) {
      error = String(err);
      if (error.includes("iniciar sesión")) onSesionExpirada();
    }
  }

  onMount(load);

  let timer;
  function onSearch() {
    clearTimeout(timer);
    timer = setTimeout(load, 250);
  }

  async function save(input) {
    if (editing?.id) await api.updateProduct(editing.id, input);
    else await api.createProduct(input);
    editing = null;
    await load();
  }

  // Primer toque pide confirmación; el segundo elimina.
  // (window.confirm no funciona de forma fiable en el WebView de Android.)
  async function remove(p) {
    if (confirmId !== p.id) {
      confirmId = p.id;
      return;
    }
    confirmId = null;
    try {
      await api.deleteProduct(p.id);
      await load();
    } catch (err) {
      error = String(err);
    }
  }
</script>

<header class="topbar">
  <div>
    <strong>Productos</strong>
    <div class="muted small">{user.nombre} · base {cfg.moneda_base}</div>
  </div>
</header>

<main class="content">
  {#if tasasViejas.length}
    <button class="alert" onclick={irATasas}>
      ⚠ {tasasViejas.length === 1 ? "La tasa" : "Las tasas"} de
      {tasasViejas.map((t) => t.moneda).join(", ")}
      {tasasViejas.some((t) => !t.tasa) ? "no se ha cargado" : "no es de hoy"}. Toca para actualizar.
    </button>
  {/if}

  <input class="search" type="search" placeholder="Buscar por nombre o SKU" bind:value={search} oninput={onSearch} />

  {#if error}<p class="error">{error}</p>{/if}

  {#if products.length === 0}
    <p class="muted center">No hay productos{search ? " que coincidan" : " todavía"}.</p>
  {:else}
    <ul class="list">
      {#each products as p (p.id)}
        <li class="card item">
          <button class="item-main" onclick={() => (editing = p)}>
            <span class="name">{p.nombre}</span>
            <span class="muted small">SKU {p.sku}</span>
            <span class="stock" class:low={p.stock === 0}>Stock: {p.stock}</span>
          </button>
          <div class="item-side">
            <span class="price">{formatMoney(p.precio, monedaInfo(p.moneda))}</span>
            {#each p.equivalentes as eq}
              <span class="equiv">{formatMoney(eq.precio, monedaInfo(eq.moneda))}</span>
            {/each}
            <button class="link danger" onclick={() => remove(p)}>
              {confirmId === p.id ? "¿Seguro? Toca otra vez" : "Eliminar"}
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</main>

<button class="fab" aria-label="Nuevo producto" onclick={() => (editing = {})}>＋</button>

{#if editing}
  <ProductForm product={editing.id ? editing : null} onSave={save} onCancel={() => (editing = null)} />
{/if}
