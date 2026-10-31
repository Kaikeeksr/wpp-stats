<script lang="ts">
  import type { Snippet } from "svelte";
  import { reducedMotion } from "../lib/theme.svelte";

  let {
    open = $bindable(false),
    align = "start",
    trigger,
    children,
  }: {
    open?: boolean;
    align?: "start" | "end" | "center";
    trigger: Snippet<[() => void, boolean]>;
    children: Snippet<[() => void]>;
  } = $props();

  let root: HTMLElement;
  const toggle = () => (open = !open);
  const close = () => (open = false);

  function pop(_: Element) {
    return {
      duration: reducedMotion ? 0 : 160,
      css: (t: number) => `opacity:${t};transform:translateY(${(1 - t) * -4}px) scale(${0.98 + t * 0.02})`,
    };
  }

  $effect(() => {
    if (!open) return;
    const down = (e: MouseEvent) => !root.contains(e.target as Node) && close();
    const key = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("mousedown", down);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("mousedown", down);
      window.removeEventListener("keydown", key);
    };
  });
</script>

<div class="anchor" bind:this={root}>
  {@render trigger(toggle, open)}
  {#if open}
    <div class="pop {align}" transition:pop>
      {@render children(close)}
    </div>
  {/if}
</div>

<style>
  .anchor { position: relative; display: inline-flex; }
  .pop {
    position: absolute; top: calc(100% + 6px); z-index: 50; min-width: 200px; padding: 6px;
    background: var(--surface); border-radius: var(--r); box-shadow: var(--float); transform-origin: top left;
  }
  .start { left: 0; }
  .end { right: 0; transform-origin: top right; }
  .center { left: 50%; translate: -50% 0; transform-origin: top center; }
  :global(.pop .item) {
    display: flex; align-items: center; gap: 10px; width: 100%; height: 32px; padding: 0 10px; border-radius: 6px;
    text-align: left; white-space: nowrap; transition: background 120ms var(--ease);
  }
  :global(.pop .item:hover) { background: var(--hover); }
  :global(.pop .item.on) { font-weight: 500; }
  :global(.pop .item kbd), :global(.pop .item .aside) { margin-left: auto; padding-left: 16px; color: var(--text-3); font-size: 12px; }
  :global(.pop hr) { border: 0; border-top: 1px solid var(--border); margin: 6px -6px; }
  :global(.pop .group-label) { padding: 6px 10px 4px; font-size: 11.5px; color: var(--text-3); }
</style>
