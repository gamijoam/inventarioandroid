<script>
  import { api } from "./api.js";

  let { onLogin } = $props();
  let username = $state("");
  let password = $state("");
  let error = $state("");
  let loading = $state(false);

  async function submit(e) {
    e.preventDefault();
    error = "";
    loading = true;
    try {
      onLogin(await api.login(username, password));
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }
</script>

<main class="login">
  <form class="card" onsubmit={submit}>
    <h1>Inventario</h1>
    <p class="muted">Inicia sesión para continuar</p>

    <label>
      Usuario
      <input bind:value={username} autocomplete="username" autocapitalize="none" required />
    </label>
    <label>
      Contraseña
      <input type="password" bind:value={password} autocomplete="current-password" required />
    </label>

    {#if error}<p class="error">{error}</p>{/if}

    <button class="primary" disabled={loading}>
      {loading ? "Entrando…" : "Entrar"}
    </button>
  </form>
</main>
