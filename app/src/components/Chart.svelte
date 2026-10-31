<script lang="ts">
  import { base, brush, echarts, type Option } from "../lib/charts";
  import { theme } from "../lib/theme.svelte";

  let {
    option,
    height,
    onrange,
  }: { option: Option; height?: number; onrange?: (i0: number, i1: number) => void } = $props();

  let el: HTMLDivElement;
  let c = $state.raw<echarts.ECharts | null>(null);

  $effect(() => {
    theme.dark;
    const inst = echarts.init(el, null, { renderer: "svg" });
    const ro = new ResizeObserver(() => inst.resize());
    ro.observe(el);
    inst.on("brushEnd", (p: any) => {
      const r = p.areas?.[0]?.coordRange as [number, number] | undefined;
      inst.dispatchAction({ type: "brush", areas: [] });
      if (r && r[1] >= r[0]) onrange?.(Math.max(0, r[0]), r[1]);
    });
    c = inst;
    return () => {
      ro.disconnect();
      inst.dispose();
    };
  });

  $effect(() => {
    if (!c) return;
    c.setOption({ ...base(), ...(onrange ? brush() : {}), ...option }, { replaceMerge: ["series", "xAxis", "yAxis"] });
    if (onrange) c.dispatchAction({ type: "takeGlobalCursor", key: "brush", brushOption: { brushType: "lineX", brushMode: "single" } });
  });
</script>

<div class="chart" class:grow={!height} bind:this={el} style:height={height ? `${height}px` : null}></div>

<style>
  .chart { width: 100%; min-width: 0; }
</style>
