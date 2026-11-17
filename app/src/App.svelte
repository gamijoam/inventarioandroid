<script>
  import { onMount } from "svelte";
  import { api } from "./lib/api.js";
  import { loadConfig } from "./lib/config.svelte.js";
  import Login from "./lib/Login.svelte";
  import Products from "./lib/Products.svelte";
  import Rates from "./lib/Rates.svelte";
  import Settings from "./lib/Settings.svelte";
  import Update from "./lib/Update.svelte";

  let user = $state(null);
  let ready = $state(false);
  let tab = $state("productos");
  let update = $state(null); // info de actualización a mostrar

  onMount(async () => {
    try {
      user = await api.currentUser();
      if (user) await loadConfig();
    } finally {
      ready = true;
    }
    // Buscar actualizaciones al abrir, sin bloquear ni molestar si no hay internet.
    try {
      const info = await api.checkUpdate();
      if (info.disponible) update = info;
    } catch {}
  });

  async function onLogin(u) {
    await loadConfig();
    user = u;
    tab = "productos";
  }

  async function logout() {
    await api.logout();
    user = null;
  }
</script>

{#if !ready}
  <div class="center muted">Cargando…</div>
{:else if user}
  <div class="page">
    {#if tab === "productos"}
      <Products {user} onSesionExpirada={logout} irATasas={() => (tab = "tasas")} />
    {:else if tab === "tasas"}
      <Rates />
    {:else}
      <Settings {user} onLogout={logout} onUpdate={(info) => (update = info)} />
    {/if}
  </div>

  <nav class="tabs">
    <button class:active={tab === "productos"} onclick={() => (tab = "productos")}>
      <span class="ico">📦</span>Productos
    </button>
    <button class:active={tab === "tasas"} onclick={() => (tab = "tasas")}>
      <span class="ico">💱</span>Tasas
    </button>
    <button class:active={tab === "ajustes"} onclick={() => (tab = "ajustes")}>
      <span class="ico">⚙️</span>Ajustes
    </button>
  </nav>
{:else}
  <Login {onLogin} />
{/if}

{#if update}
  <Update info={update} onClose={() => (update = null)} />
{/if}
