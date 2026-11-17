<script>
  import { onMount } from "svelte";
  import { api } from "./api.js";
  import { cfg, loadConfig, setConfig } from "./config.svelte.js";

  let { user, onLogout, onUpdate } = $props();

  let error = $state("");
  let info = $state(null);
  let buscando = $state(false);
  let msgUpdate = $state("");
  let nuevoTipo = $state("");
  let editandoTipo = $state(null); // {id, nombre}
  let mostrarTodas = $state(false);
  let pass = $state({ actual: "", nueva: "", msg: "", error: "" });

  onMount(async () => {
    info = await api.appInfo();
  });

  async function run(fn) {
    error = "";
    try {
      await fn();
    } catch (e) {
      error = String(e);
    }
  }

  const cambiarBase = (codigo) => run(async () => setConfig(await api.setMonedaBase(codigo)));
  const toggle = (m) => run(async () => setConfig(await api.setMonedaActiva(m.codigo, !m.activa)));

  const crearTipo = () =>
    run(async () => {
      await api.crearTipoTasa(nuevoTipo);
      nuevoTipo = "";
      await loadConfig();
    });
  const guardarTipo = () =>
    run(async () => {
      await api.renombrarTipoTasa(editandoTipo.id, editandoTipo.nombre);
      editandoTipo = null;
      await loadConfig();
    });
  const defecto = (t) => run(async () => { await api.setTipoTasaDefecto(t.id); await loadConfig(); });
  const eliminarTipo = (t) => run(async () => { await api.eliminarTipoTasa(t.id); await loadConfig(); });

  async function buscarUpdate() {
    buscando = true;
    msgUpdate = "";
    try {
      const u = await api.checkUpdate();
      if (u.disponible) onUpdate(u);
      else msgUpdate = "Ya tienes la última versión.";
    } catch (e) {
      msgUpdate = String(e);
    } finally {
      buscando = false;
    }
  }

  async function cambiarPass(e) {
    e.preventDefault();
    pass.msg = pass.error = "";
    try {
      await api.changePassword(pass.actual, pass.nueva);
      pass = { actual: "", nueva: "", msg: "Contraseña actualizada", error: "" };
    } catch (err) {
      pass.error = String(err);
    }
  }

  let visibles = $derived(mostrarTodas ? cfg.monedas : cfg.monedas.filter((m) => m.activa));
</script>

<header class="topbar">
  <div>
    <strong>Ajustes</strong>
    <div class="muted small">{user.nombre}</div>
  </div>
  <button onclick={onLogout}>Salir</button>
</header>

<main class="content settings">
  {#if error}<p class="error sticky-error">{error}</p>{/if}

  <section class="card">
    <h3>Monedas</h3>
    <label>
      Moneda base (inventario y reportes)
      <select value={cfg.moneda_base} onchange={(e) => cambiarBase(e.currentTarget.value)}>
        {#each cfg.monedas as m}<option value={m.codigo}>{m.codigo} · {m.nombre}</option>{/each}
      </select>
    </label>
    <p class="muted small">Las tasas se cargan respecto a la moneda base. Si la cambias, tendrás que cargar tasas nuevas.</p>

    <p class="label">Monedas activas</p>
    <ul class="checks">
      {#each visibles as m (m.codigo)}
        <li>
          <label class="check">
            <input type="checkbox" checked={m.activa} disabled={m.codigo === cfg.moneda_base} onchange={() => toggle(m)} />
            <span><b>{m.codigo}</b> {m.nombre} <span class="muted">({m.simbolo})</span></span>
          </label>
        </li>
      {/each}
    </ul>
    <button class="link" onclick={() => (mostrarTodas = !mostrarTodas)}>
      {mostrarTodas ? "Ver solo las activas" : "Agregar otra moneda…"}
    </button>
  </section>

  <section class="card">
    <h3>Tipos de tasa</h3>
    <p class="muted small">Ej. BCV y Paralelo. Los precios equivalentes usan el tipo marcado por defecto.</p>
    <ul class="tipos">
      {#each cfg.tipos_tasa as t (t.id)}
        <li>
          {#if editandoTipo?.id === t.id}
            <form class="row" onsubmit={(e) => { e.preventDefault(); guardarTipo(); }}>
              <input bind:value={editandoTipo.nombre} />
              <button class="primary">OK</button>
            </form>
          {:else}
            <span class="tipo-nombre">{t.nombre} {#if t.es_defecto}<span class="badge">por defecto</span>{/if}</span>
            <span class="tipo-acciones">
              {#if !t.es_defecto}<button class="link" onclick={() => defecto(t)}>Usar por defecto</button>{/if}
              <button class="link" onclick={() => (editandoTipo = { id: t.id, nombre: t.nombre })}>Renombrar</button>
              {#if !t.es_defecto}<button class="link danger" onclick={() => eliminarTipo(t)}>Eliminar</button>{/if}
            </span>
          {/if}
        </li>
      {/each}
    </ul>
    <form class="row" onsubmit={(e) => { e.preventDefault(); crearTipo(); }}>
      <input placeholder="Nuevo tipo, ej. Paralelo" bind:value={nuevoTipo} required />
      <button>Agregar</button>
    </form>
  </section>

  <section class="card">
    <h3>Cambiar contraseña</h3>
    <form onsubmit={cambiarPass}>
      <label>Actual <input type="password" bind:value={pass.actual} required /></label>
      <label>Nueva (mínimo 6) <input type="password" bind:value={pass.nueva} required /></label>
      {#if pass.error}<p class="error">{pass.error}</p>{/if}
      {#if pass.msg}<p class="ok small">{pass.msg}</p>{/if}
      <button>Cambiar contraseña</button>
    </form>
  </section>

  <section class="card">
    <h3>Acerca de</h3>
    {#if info}
      <p>Inventario <b>v{info.version}</b></p>
      <p class="muted small">Base de datos v{info.esquema_bd} · {info.plataforma}</p>
    {/if}
    <button class="primary" disabled={buscando} onclick={buscarUpdate}>
      {buscando ? "Buscando…" : "Buscar actualizaciones"}
    </button>
    {#if msgUpdate}<p class="muted small">{msgUpdate}</p>{/if}
  </section>
</main>
