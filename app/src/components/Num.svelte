<script lang="ts">
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";
  import { reducedMotion } from "../lib/theme.svelte";

  let { value, format }: { value: number; format: (v: number) => string } = $props();
  const t = new Tween(0, { duration: reducedMotion ? 0 : 650, easing: cubicOut });
  $effect(() => {
    t.target = value;
  });
</script>

<span class="num">{format(t.current)}</span>
