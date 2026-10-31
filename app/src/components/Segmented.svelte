<script lang="ts" generics="T">
  import type { Snippet } from "svelte";

  let {
    items,
    value,
    onchange,
    label,
    size = "md",
  }: {
    items: { value: T; title?: string }[];
    value: T;
    onchange: (v: T) => void;
    label: Snippet<[T, number]>;
    size?: "sm" | "md";
  } = $props();

  let buttons: HTMLButtonElement[] = $state([]);
  let ind = $state({ x: 0, w: 0, on: false });
  let ready = $state(false);

  function measure() {
    const i = items.findIndex((it) => it.value === value);
    const b = buttons[i];
    ind = b ? { x: b.offsetLeft, w: b.offsetWidth, on: true } : { ...ind, on: false };
  }

  $effect(() => {
    value;
    items;
    measure();
    requestAnimationFrame(() => (ready = true));
  });

  $effect(() => {
    const ro = new ResizeObserver(measure);
    buttons.forEach((b) => b && ro.observe(b));
    document.fonts?.ready.then(measure);
    return () => ro.disconnect();
  });
</script>

<div class="seg {size}" role="tablist">
  <i class="ind" class:ready class:on={ind.on} style:transform="translateX({ind.x}px)" style:width="{ind.w}px"></i>
  {#each items as it, i (i)}
    <button
      bind:this={buttons[i]}
      role="tab"
      aria-selected={it.value === value}
      class:on={it.value === value}
      title={it.title}
      onclick={() => onchange(it.value)}>{@render label(it.value, i)}</button>
  {/each}
</div>

<style>
  .seg { position: relative; display: inline-flex; padding: 3px; gap: 2px; border-radius: 10px; background: var(--active); }
  .ind {
    position: absolute; top: 3px; bottom: 3px; left: 0; border-radius: 7px; background: var(--surface);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08), 0 0 0 0.5px rgba(0, 0, 0, 0.06); opacity: 0;
  }
  :global([data-theme="dark"]) .ind { background: #3a3a3a; box-shadow: none; }
  .ind.on { opacity: 1; }
  .ind.ready { transition: transform 280ms var(--ease), width 280ms var(--ease); }
  button {
    position: relative; display: inline-flex; align-items: center; gap: 7px; height: 26px; padding: 0 12px; border-radius: 7px;
    color: var(--text-2); font-weight: 500; white-space: nowrap; max-width: 200px; transition: color 150ms var(--ease);
  }
  button:hover, button.on { color: var(--text); }
  .sm button { height: 24px; padding: 0 10px; font-size: 12.5px; }
</style>
