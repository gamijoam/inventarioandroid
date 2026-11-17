<script>
  // Aviso de actualización. `info` viene de check_update.
  import { api } from "./api.js";

  let { info, onClose } = $props();

  let estado = $state("listo"); // listo | descargando | permiso | instalando | error
  let progreso = $state(0);
  let error = $state("");

  async function actualizar() {
    error = "";
    estado = "descargando";
    progreso = 0;
    try {
      const res = await api.installUpdate(info.url, info.sha256, (p) => {
        progreso = p.total ? Math.round((p.descargado / p.total) * 100) : 0;
      });
      estado = res; // "instalando" o "permiso"
    } catch (e) {
      error = String(e);
      estado = "error";
    }
  }
</script>

<div class="backdrop center-modal">
  <div class="card update">
    <h2>Nueva versión {info.version}</h2>
    <p class="muted small">Tienes la {info.actual}{info.obligatoria ? " · esta actualización es obligatoria" : ""}</p>
    {#if info.notas}<p class="notas">{info.notas}</p>{/if}

    {#if !info.puede_instalar}
      <p class="muted">Descarga la nueva versión desde la página de Releases del proyecto.</p>
    {:else if estado === "descargando"}
      <div class="bar"><div style="width: {progreso}%"></div></div>
      <p class="muted small center-text">Descargando… {progreso}%</p>
    {:else if estado === "permiso"}
      <p class="warn">
        Android necesita que permitas a Inventario <b>instalar apps</b>. Activa el permiso en la pantalla
        que se abrió, vuelve aquí y toca <b>Actualizar</b> otra vez.
      </p>
    {:else if estado === "instalando"}
      <p>Confirma en la ventana de Android para terminar. La app se cerrará y se abrirá actualizada.</p>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}

    <div class="row actions">
      {#if !info.obligatoria}
        <button onclick={onClose}>Más tarde</button>
      {/if}
      {#if info.puede_instalar}
        <button class="primary" disabled={estado === "descargando"} onclick={actualizar}>
          {estado === "descargando" ? "Descargando…" : "Actualizar"}
        </button>
      {/if}
    </div>
  </div>
</div>
