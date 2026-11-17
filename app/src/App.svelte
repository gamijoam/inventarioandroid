<script>
  import { onMount } from "svelte";
  import { api } from "./lib/api.js";
  import Login from "./lib/Login.svelte";
  import Products from "./lib/Products.svelte";

  let user = $state(null);
  let ready = $state(false);

  onMount(async () => {
    try {
      user = await api.currentUser();
    } finally {
      ready = true;
    }
  });

  async function logout() {
    await api.logout();
    user = null;
  }
</script>

{#if !ready}
  <div class="center muted">Cargando…</div>
{:else if user}
  <Products {user} onLogout={logout} />
{:else}
  <Login onLogin={(u) => (user = u)} />
{/if}
