<script lang="ts">
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import FilterBar from "./FilterBar.svelte";
  import StatCards from "./StatCards.svelte";
  import Audios from "../tabs/Audios.svelte";
  import Highlights from "../tabs/Highlights.svelte";
  import Hours from "../tabs/Hours.svelte";
  import Media from "../tabs/Media.svelte";
  import Messages from "../tabs/Messages.svelte";
  import Overview from "../tabs/Overview.svelte";
  import { app } from "../lib/store.svelte";
  import { reducedMotion } from "../lib/theme.svelte";

  const TAB = [Overview, Messages, Audios, Media, Hours, Highlights];
  const Tab = $derived(TAB[app.view.tab]);
  let scroller: HTMLElement;
  $effect(() => {
    void app.view.tab;
    scroller?.scrollTo({ top: 0 });
  });
</script>

<div class="board" bind:this={scroller}>
  <div class="filters"><div class="inner"><FilterBar /></div></div>
  <main class="inner content">
    {#if app.stats?.from}
      <StatCards s={app.stats} />
      {#key app.view.tab}
        <div in:fly={{ y: 8, duration: reducedMotion ? 0 : 320, easing: cubicOut }}>
          <Tab s={app.stats} />
        </div>
      {/key}
    {:else}
      <p class="empty">Nenhuma mensagem no período.</p>
    {/if}
  </main>
</div>

<style>
  .board { flex: 1; overflow-y: auto; overflow-x: hidden; scrollbar-width: none; }
  .board::-webkit-scrollbar { display: none; }
  .inner { width: min(1280px, 100% - 48px); margin: 0 auto; }
  .filters { position: sticky; top: 0; z-index: 10; padding: 4px 0 12px; background: var(--bg); }
  .content { display: grid; grid-template-columns: minmax(0, 1fr); gap: 16px; padding: 4px 0 32px; }
</style>
