<script lang="ts">
  import Num from "./Num.svelte";
  import Who from "./Who.svelte";
  import { FILE_KINDS, K, type Stats, type Totals } from "../lib/api";
  import { audiosN, fmtDec, fmtDur, fmtInt, fmtSize, pct, plural } from "../lib/fmt";
  import { groups, gsum, isGroup, shown, sum } from "../lib/people";

  let { s }: { s: Stats } = $props();

  const multi = $derived(shown(s).length > 1);
  const gs = $derived(groups(s, isGroup(s) ? 5 : 8));

  const cards = $derived([
    {
      label: "Mensagens",
      f: (t: Totals) => t.messages,
      fmt: fmtInt,
      sub: `${fmtDec(s.total.messages_per_day)} por dia`,
    },
    {
      label: "Tempo de áudio",
      f: (t: Totals) => t.dur_ms[K.Audio],
      fmt: fmtDur,
      sub: audiosN(s.total.count[K.Audio]),
    },
    {
      label: "Mídias",
      f: (t: Totals) => sum(t.media_bytes),
      fmt: fmtSize,
      sub: `${plural(sum(FILE_KINDS.map((k) => s.total.count[k])), "arquivo", "arquivos")} (áudios entram na conta de mídia)`,
    },
  ]);
</script>

<section class="cards">
  {#each cards as c (c.label)}
    {@const all = c.f(s.total)}
    <div class="card">
      <div class="label">{c.label}</div>
      <div class="big hero"><Num value={all} format={c.fmt} /></div>
      <div class="sub">{c.sub}</div>
      {#if multi}
        <div class="share">
          {#each gs as g (g.name)}
            <i style:flex-grow={gsum(g, (p) => c.f(s.per_person[p]))} style:background={g.color}></i>
          {/each}
        </div>
        <div class="split">
          {#each gs as g (g.name)}
            {@const v = gsum(g, (p) => c.f(s.per_person[p]))}
            <div>
              <Who name={g.name} color={g.color} />
              <span class="num v">{c.fmt(v)}<span class="pct">{pct(v, all)}</span></span>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</section>

<style>
  .cards { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 16px; }
  .hero { font-size: 32px; margin: 4px 0 2px; }
  .share { display: flex; gap: 3px; height: 6px; margin: 16px 0 12px; }
  .share i { flex: 1 1 0; min-width: 0; border-radius: 3px; transition: flex-grow 600ms var(--ease); }
  .split { display: grid; gap: 4px; }
  .split div { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .v { font-size: 13px; white-space: nowrap; }
  .pct { display: inline-block; min-width: 40px; text-align: right; color: var(--text-3); font-size: 12px; }
</style>
