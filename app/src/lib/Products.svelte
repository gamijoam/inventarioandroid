<script>
  import { onMount } from "svelte";
  import { api } from "./api.js";
  import ProductForm from "./ProductForm.svelte";

  let { user, onLogout } = $props();

  let products = $state([]);
  let search = $state("");
  let error = $state("");
  let editing = $state(null); // null = cerrado, {} = nuevo, producto = editar
  let confirmId = $state(null); // producto pendiente de confirmar eliminación

  const money = new Intl.NumberFormat("es", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

  async function load() {
    try {
      error = "";
      products = await api.listProducts(search);
    } catch (err) {
      error = String(err);
      if (error.includes("iniciar sesión")) onLogout();
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
    <div class="muted small">{user.nombre}</div>
  </div>
  <button onclick={onLogout}>Salir</button>
</header>

<main class="content">
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
          </button>
          <div class="item-side">
            <span class="price">{money.format(p.precio)}</span>
            <span class="stock" class:low={p.stock === 0}>Stock: {p.stock}</span>
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
