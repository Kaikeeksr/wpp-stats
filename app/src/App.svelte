<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Download } from "lucide";
  import { fade, fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import Dashboard from "./components/Dashboard.svelte";
  import Home from "./components/Home.svelte";
  import Icon from "./components/Icon.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import { api } from "./lib/api";
  import { app, dismissToast, openFolder, pickZip, refresh, setTab, start } from "./lib/store.svelte";
  import { reducedMotion } from "./lib/theme.svelte";

  let over = $state(false);
  let dragName = $state("");
  let exportOpen = $state(false);
  let ready = $state(false);
  const dur = (ms: number) => (reducedMotion ? 0 : ms);

  $effect(() => {
    (async () => {
      await start();
      ready = true;
      requestAnimationFrame(() => getCurrentWindow().show().catch(() => {}));
      await listen<number>("progress", (e) => (app.progress = e.payload));
      await listen("extras", () => app.screen === "dashboard" && refresh());
      await getCurrentWebview().onDragDropEvent((e) => {
        const t = e.payload.type;
        if (t === "enter") dragName = e.payload.paths[0]?.split(/[\\/]/).pop() ?? "";
        over = t === "enter" || t === "over";
        if (t === "drop" && e.payload.paths.length && app.screen !== "processing") openFolder(e.payload.paths[0]);
      });
      const initial = await api.initialFolder();
      if (initial) openFolder(initial);
    })();
  });

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape" && over) over = false;
    else if (e.key === "Escape" && app.screen === "processing") api.cancel();
    if (!e.ctrlKey || e.altKey) return;
    const k = e.key.toLowerCase();
    if (k === "o" && app.screen !== "processing") {
      e.preventDefault();
      pickZip();
    } else if (app.screen === "dashboard" && k === "e") {
      e.preventDefault();
      exportOpen = !exportOpen;
    } else if (app.screen === "dashboard" && /^[1-6]$/.test(k)) {
      e.preventDefault();
      setTab(+k - 1);
    }
  }
</script>

<svelte:window onkeydown={onkey} />

{#if ready}
  <div class="shell">
    <TitleBar bind:exportOpen />
    {#key app.screen === "dashboard"}
      <div class="screen" in:fly={{ y: 6, duration: dur(320), easing: cubicOut }}>
        {#if app.screen === "dashboard"}
          <Dashboard />
        {:else}
          <Home />
        {/if}
      </div>
    {/key}
    {#if over && app.screen !== "processing"}
      <div class="overlay" transition:fade={{ duration: dur(150) }}>
        <div class="frame">
          <span class="drop-tile"><Icon icon={Download} size={28} stroke={2} /></span>
          <p class="drop-title">Solte para analisar</p>
          {#if dragName}<p class="drop-name">{dragName}</p>{/if}
        </div>
      </div>
    {/if}
    {#if app.toast}
      <div class="toast" class:bad={app.toast.bad} transition:fly={{ y: 12, duration: dur(240), easing: cubicOut }}>
        {app.toast.text}
        {#if app.toast.action}
          {@const a = app.toast.action}
          <button class="toast-act" onclick={() => (a.run(), dismissToast())}>{a.label}</button>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .shell { height: 100vh; display: flex; flex-direction: column; position: relative; }
  .screen { flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden; }
  .screen > :global(main) { flex: 1; overflow-y: auto; scrollbar-width: none; }
  .toast {
    position: fixed; left: 50%; bottom: 24px; translate: -50% 0; z-index: 60; max-width: 560px;
    padding: 10px 16px; border-radius: 10px; background: var(--ink); color: var(--on-ink); font-weight: 500; font-size: 13px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  }
  .toast.bad { background: var(--bad); color: #fff; }
  .toast { display: flex; align-items: center; gap: 16px; }
  .toast-act { font-weight: 600; text-decoration: underline; text-underline-offset: 2px; }
  .overlay { position: fixed; inset: 0; z-index: 50; padding: 16px; display: flex; background: var(--overlay); pointer-events: none; }
  .frame {
    flex: 1; border: 2px dashed var(--text); border-radius: 12px; color: var(--text);
    display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 16px; text-align: center;
  }
  .drop-tile { width: 64px; height: 64px; border-radius: 12px; background: var(--text); color: var(--on-ink); display: grid; place-items: center; }
  .drop-title { font-size: 32px; font-weight: 700; letter-spacing: -0.02em; line-height: 1.2; }
  .drop-name { font-size: 14px; color: var(--text-2); }
</style>
