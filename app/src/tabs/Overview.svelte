<script lang="ts">
  import Chart from "../components/Chart.svelte";
  import Segmented from "../components/Segmented.svelte";
  import Who from "../components/Who.svelte";
  import type { Stats } from "../lib/api";
  import { fmtDec, fmtInt, pct } from "../lib/fmt";
  import { bucketRange, cumulative, defaultGrain, donut, GRAIN_LABELS, grains, regroup, trend, type Grain } from "../lib/options";
  import { groups, gsum, sum } from "../lib/people";
  import { setPeriod } from "../lib/store.svelte";

  let { s }: { s: Stats } = $props();

  let picked = $state<Grain | null>(null);
  const options = $derived(grains(s.timeline));
  const grain = $derived(picked && options.includes(picked) ? picked : defaultGrain(s.timeline));
  const tl = $derived(regroup(s.timeline, grain));
  const unit = $derived({ dia: "dia", semana: "semana", mes: "mês" }[grain]);
  const gs = $derived(groups(s));
  const count = (g: { members: number[] }) => gsum(g, (p) => s.per_person[p].messages);
</script>

<div class="page">
  <div class="card s6">
    <div class="card-head">
      <h3>Mensagens por {unit}</h3>
      <div class="legend">
        <span class="who"><i class="dash"></i>média de {fmtDec(sum(tl.total) / Math.max(1, tl.total.length))}</span>
        {#if options.length > 1}
          <Segmented size="sm" items={options.map((g) => ({ value: g }))} value={grain} onchange={(g) => (picked = g)}>
            {#snippet label(g)}{GRAIN_LABELS[g]}{/snippet}
          </Segmented>
        {/if}
      </div>
    </div>
    <Chart height={240} option={trend(tl)} onrange={(i0, i1) => setPeriod(...bucketRange(tl.labels, s.to!, i0, i1))} />
  </div>

  <div class="card s4">
    <div class="card-head">
      <h3>Mensagens acumuladas</h3>
      {#if gs.length > 1}<div class="legend">{#each gs as g (g.name)}<Who name={g.name} color={g.color} />{/each}</div>{/if}
    </div>
    <Chart
      height={250}
      option={cumulative(s)}
      onrange={(i0, i1) => setPeriod(s.progression.labels[i0], s.progression.labels[Math.min(i1, s.progression.labels.length - 1)])} />
  </div>

  <div class="card fill s2">
    <div class="card-head"><h3>Quem mandou mais</h3></div>
    <Chart option={donut(gs, count)} />
    <div class="rows">
      {#each gs as g (g.name)}
        <div><Who name={g.name} color={g.color} /><span class="num">{fmtInt(count(g))}<span class="pct">{pct(count(g), s.total.messages)}</span></span></div>
      {/each}
    </div>
  </div>
</div>

<style>
  .legend { align-items: center; }
  .rows { display: grid; gap: 2px; margin-top: 12px; }
  .rows div { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
  .pct { display: inline-block; min-width: 40px; text-align: right; color: var(--text-3); font-size: 12px; }
</style>
