<script lang="ts">
  import type { Snippet } from "svelte";

  let { label, value, sub = "", children }: { label: string; value: string; sub?: string; children?: Snippet } = $props();
</script>

<div class="card stat">
  <div class="label">{label}</div>
  <div class="big">{value}</div>
  <div class="sub num">{sub}</div>
  {@render children?.()}
</div>

<style>
  .stat .big { font-size: 22px; margin: 4px 0 0; }
</style>
