<script>
  import { onMount } from "svelte";
  import { api } from "./api.js";
  import { cfg, monedaInfo } from "./config.svelte.js";
  import { formatDate, formatRate } from "./money.js";

  let vigentes = $state([]);
  let valores = $state({}); // clave "MONEDA:tipo" -> texto del input
  let mensajes = $state({}); // clave -> {ok|error}
  let abierto = $state(null); // clave con historial visible
  let historial = $state([]);
  let error = $state("");

  const clave = (t) => `${t.moneda}:${t.tipo_id}`;

  async function load() {
    try {
      vigentes = await api.tasasVigentes();
    } catch (e) {
      error = String(e);
    }
  }
  onMount(load);

  async function guardar(t) {
    const k = clave(t);
    mensajes[k] = null;
    try {
      await api.registrarTasa(t.moneda, t.tipo_id, valores[k] ?? "");
      valores[k] = "";
      mensajes[k] = { ok: "Tasa guardada" };
      await load();
      if (abierto === k) await verHistorial(t, true);
    } catch (e) {
      mensajes[k] = { error: String(e) };
    }
  }

  async function verHistorial(t, forzar = false) {
    const k = clave(t);
    if (abierto === k && !forzar) {
      abierto = null;
      return;
    }
    historial = await api.historialTasas(t.moneda, t.tipo_id);
    abierto = k;
  }
</script>

<header class="topbar">
  <div>
    <strong>Tasas de cambio</strong>
    <div class="muted small">Respecto a la moneda base ({cfg.moneda_base})</div>
  </div>
</header>

<main class="content">
  {#if error}<p class="error">{error}</p>{/if}

  {#if vigentes.length === 0}
    <div class="card">
      <p>Solo tienes activa la moneda base ({cfg.moneda_base}), así que no necesitas tasas.</p>
      <p class="muted small">Para trabajar con otra moneda (por ejemplo Bs), actívala en <b>Ajustes → Monedas</b>.</p>
    </div>
  {/if}

  <div class="list">
    {#each vigentes as t (clave(t))}
      {@const m = monedaInfo(t.moneda)}
      {@const k = clave(t)}
      <section class="card rate">
        <div class="rate-head">
          <div>
            <strong>{t.moneda}</strong> · {t.tipo_nombre}
            {#if t.es_defecto}<span class="badge">por defecto</span>{/if}
          </div>
          {#if t.desactualizada}<span class="badge warn-badge">{t.tasa ? "no es de hoy" : "sin tasa"}</span>{/if}
        </div>

        {#if t.tasa}
          <p class="rate-value">1 {cfg.moneda_base} = {m?.simbolo} {formatRate(t.tasa.valor)}</p>
          <p class="muted small">Cargada el {formatDate(t.tasa.fecha)}</p>
        {:else}
          <p class="muted">Todavía no se ha cargado ninguna tasa.</p>
        {/if}

        <form class="row rate-form" onsubmit={(e) => { e.preventDefault(); guardar(t); }}>
          <input inputmode="decimal" placeholder="Tasa de hoy, ej. 36,50" bind:value={valores[k]} required />
          <button class="primary">Guardar</button>
        </form>
        {#if mensajes[k]?.ok}<p class="ok small">{mensajes[k].ok}</p>{/if}
        {#if mensajes[k]?.error}<p class="error">{mensajes[k].error}</p>{/if}

        <button class="link" onclick={() => verHistorial(t)}>
          {abierto === k ? "Ocultar historial" : "Ver historial"}
        </button>
        {#if abierto === k}
          <ul class="hist">
            {#each historial as h}
              <li><span>{formatDate(h.fecha)}</span><span>{m?.simbolo} {formatRate(h.valor)}</span></li>
            {:else}
              <li class="muted">Sin registros</li>
            {/each}
          </ul>
        {/if}
      </section>
    {/each}
  </div>
</main>
