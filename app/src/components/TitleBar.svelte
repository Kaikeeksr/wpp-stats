<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { ChevronDown, Copy, Download, House, Minus, Settings2, Square, X } from "lucide";
  import Icon from "./Icon.svelte";
  import Popover from "./Popover.svelte";
  import Segmented from "./Segmented.svelte";
  import { api } from "../lib/api";
  import { crumbs, folderName } from "../lib/fmt";
  import { app, clearRecents, exportAs, goHome, openFolder, pickFolder, pickZip, setCache, setTab, setThemePref, TABS } from "../lib/store.svelte";

  let { exportOpen = $bindable(false) }: { exportOpen?: boolean } = $props();

  const win = (() => {
    try {
      return getCurrentWindow();
    } catch {
      return null;
    }
  })();
  let maximized = $state(false);
  $effect(() => {
    const sync = () => win?.isMaximized().then((m) => (maximized = m)).catch(() => {});
    sync();
    const off = win?.onResized(sync);
    return () => void off?.then((f) => f());
  });

  let cleared = $state(false);
  const recents = $derived((app.config.recent_folders ?? []).filter((p) => p !== app.opened?.path));
  const dash = $derived(app.screen === "dashboard" && app.opened);
</script>

<header class="bar" data-tauri-drag-region>
  <div class="left" data-tauri-drag-region>
    {#if dash && app.opened}
      <button class="icon-btn" onclick={goHome} aria-label="Voltar ao início" title="Início"><Icon icon={House} size={16} /></button>
      <Popover align="start">
        {#snippet trigger(toggle, open)}
          <button class="folder" class:on={open} onclick={toggle} title={app.opened!.path}>
            <span class="fname">{folderName(app.opened!.path)}</span>
            <Icon icon={ChevronDown} size={14} />
          </button>
        {/snippet}
        {#snippet children(close)}
          <div class="path">{crumbs(app.opened!.path, app.opened!.home).join("  ›  ")}</div>
          <hr />
          {#if recents.length}
            <div class="group-label">Recentes</div>
            {#each recents as p (p)}
              <button class="item" title={p} onclick={() => (close(), openFolder(p))}>{folderName(p)}</button>
            {/each}
            <hr />
          {/if}
          <button class="item" onclick={() => (close(), pickZip())}>Abrir .zip<kbd>Ctrl+O</kbd></button>
          <button class="item" onclick={() => (close(), pickFolder())}>Abrir pasta</button>
        {/snippet}
      </Popover>
    {:else}
      <div class="brand" data-tauri-drag-region>
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M4 20V10" /><path d="M10 20V4" /><path d="M16 20v-7" /><path d="M22 20H2" />
        </svg>
        WPP Stats
      </div>
    {/if}
  </div>

  {#if dash}
    <nav class="center">
      <Segmented items={TABS.map((t, i) => ({ value: i, title: `Ctrl+${i + 1}` }))} value={app.view.tab} onchange={setTab}>
        {#snippet label(i)}{TABS[i]}{/snippet}
      </Segmented>
    </nav>
  {/if}

  <div class="right" data-tauri-drag-region>
    {#if dash}
      <Popover align="end" bind:open={exportOpen}>
        {#snippet trigger(toggle, open)}
          <button class="btn quiet" class:on={open} onclick={toggle} title="Ctrl+E"><Icon icon={Download} size={15} />Exportar</button>
        {/snippet}
        {#snippet children(close)}
          {#each [["pdf", "PDF", "relatório"], ["pdf-anonimo", "PDF anônimo", "Pessoa A, B…"], ["csv", "CSV", "tabelas"], ["json", "JSON", "dados"]] as const as [f, name, aside] (f)}
            <button class="item" onclick={() => (close(), exportAs(f))}>{name}<span class="aside">{aside}</span></button>
          {/each}
        {/snippet}
      </Popover>
    {/if}
    <Popover align="end">
      {#snippet trigger(toggle, open)}
        <button class="icon-btn settings-btn" class:on={open} onclick={toggle} aria-label="Configurações" title="Configurações"><Icon icon={Settings2} size={18} stroke={2} /></button>
      {/snippet}
      {#snippet children()}
        <div class="settings">
          <div class="group-label">Tema</div>
          <div class="row">
            {#each [["system", "Sistema"], ["light", "Claro"], ["dark", "Escuro"]] as const as [v, l] (v)}
              <button class="opt" class:on={(app.config.theme ?? "system") === v} onclick={() => setThemePref(v)}>{l}</button>
            {/each}
          </div>
          <hr />
          <label class="item switch">
            Usar cache
            <input type="checkbox" checked={app.config.cache_enabled !== false} onchange={(e) => setCache(e.currentTarget.checked)} />
            <i></i>
          </label>
          <button class="item" disabled={cleared} onclick={async () => (await api.clearCache().catch(() => {}), (cleared = true))}>
            {cleared ? "Cache limpo" : "Limpar cache"}
          </button>
          <button class="item" disabled={!(app.config.recent_folders ?? []).length} onclick={clearRecents}>Limpar recentes</button>
        </div>
      {/snippet}
    </Popover>
    <div class="win">
      <button onclick={() => win?.minimize()} aria-label="Minimizar"><Icon icon={Minus} size={15} /></button>
      <button onclick={() => win?.toggleMaximize()} aria-label="Maximizar"><Icon icon={maximized ? Copy : Square} size={12} /></button>
      <button class="close" onclick={() => win?.close()} aria-label="Fechar"><Icon icon={X} size={16} /></button>
    </div>
  </div>
</header>

<style>
  .bar {
    position: relative; z-index: 20; height: 48px; flex: none; display: grid; grid-template-columns: minmax(0, 1fr) auto minmax(max-content, 1fr);
    align-items: center; padding-left: 12px; gap: 16px;
  }
  .left, .right { display: flex; align-items: center; gap: 4px; min-width: 0; height: 100%; }
  .right { justify-content: flex-end; grid-column: 3; }
  .folder {
    display: inline-flex; align-items: center; gap: 6px; height: 30px; padding: 0 8px 0 10px; border-radius: var(--r-sm);
    font-weight: 600; letter-spacing: -0.01em; color: var(--text); max-width: 280px; transition: background 150ms var(--ease);
  }
  .folder :global(svg) { color: var(--text-3); }
  .folder:hover, .folder.on { background: var(--hover); }
  .fname { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .path { padding: 6px 10px; font-size: 12px; color: var(--text-2); max-width: 360px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .icon-btn.on { background: var(--hover); color: var(--text); }
  .settings-btn { width: 40px; height: 40px; }
  .settings-btn:hover, .settings-btn.on { background: var(--surface-2); }
  .brand { display: flex; align-items: center; gap: 8px; padding-left: 8px; font-size: 14px; font-weight: 600; color: var(--text); }

  .win { display: flex; height: 100%; margin-left: 8px; }
  .win button { width: 46px; height: 100%; display: grid; place-items: center; color: var(--text-2); transition: background 120ms, color 120ms; }
  .win button:hover { background: var(--hover); color: var(--text); }
  .win .close:hover { background: #c42b1c; color: #fff; }

  .settings { width: 240px; }
  .row { display: flex; gap: 2px; padding: 3px; margin: 0 4px 4px; border-radius: 9px; background: var(--active); }
  .opt { flex: 1; height: 26px; border-radius: 6px; font-size: 12.5px; color: var(--text-2); font-weight: 500; transition: background 150ms, color 150ms; }
  .opt.on { background: var(--surface); color: var(--text); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08); }
  :global([data-theme="dark"]) .opt.on { background: #3a3a3a; }
  .switch { justify-content: space-between; }
  .switch input { position: absolute; opacity: 0; pointer-events: none; }
  .switch i { width: 30px; height: 18px; border-radius: 9px; background: var(--border-2); position: relative; transition: background 200ms var(--ease); }
  .switch i::after {
    content: ""; position: absolute; top: 2px; left: 2px; width: 14px; height: 14px; border-radius: 50%; background: #fff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2); transition: transform 200ms var(--ease);
  }
  .switch input:checked + i { background: var(--ink); }
  .switch input:checked + i::after { transform: translateX(12px); }
  :global([data-theme="dark"]) .switch input:checked + i::after { background: var(--on-ink); }
</style>
