<script lang="ts">
  import { createElement, type IconNode } from "lucide";

  let { icon, size = 16, stroke = 1.75 }: { icon: IconNode; size?: number; stroke?: number } = $props();
  const svg = $derived(createElement(icon, { width: size, height: size, "stroke-width": stroke }).outerHTML);
</script>

{@html svg}
