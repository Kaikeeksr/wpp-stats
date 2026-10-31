<script lang="ts">
  import Segmented from "../components/Segmented.svelte";
  import Stat from "../components/Stat.svelte";
  import Sticker from "../components/Sticker.svelte";
  import type { DayValue, Run, Stats } from "../lib/api";
  import { audiosN, br, daysN, fmtDur, mediaN, msgs, plural, WEEKDAYS, WEEKDAYS_SHORT } from "../lib/fmt";
  import { weekdayName } from "../lib/labels";
  import { colorOf, shown, sum } from "../lib/people";

  let { s }: { s: Stats } = $props();
  const h = $derived(s.highlights);

  const METRICS = [
    { label: "Mensagens", list: () => h.messages, fmt: msgs },
    { label: "Áudios", list: () => h.audios, fmt: audiosN },
    { label: "Tempo de áudio", list: () => h.audio_ms, fmt: fmtDur },
    { label: "Mídias", list: () => h.media, fmt: mediaN },
  ];
  let metric = $state(0);
  const m = $derived(METRICS[metric]);
  const top = $derived(m.list().filter((d: DayValue) => d.value > 0));

  const byHour = $derived([...Array(24).keys()].map((hr) => sum(s.heatmap_total.map((row) => row[hr]))));
  const byWeekday = $derived(s.heatmap_total.map(sum));
  const peak = (v: number[]) => v.indexOf(Math.max(...v));
  const accent = $derived(shown(s).length === 1 ? colorOf(shown(s)[0]) : "var(--ink)");
  const times = (v: number) => plural(v, "vez", "vezes");
</script>

{#snippet spark(values: number[], labels: string[], every: number)}
  {@const max = Math.max(1, ...values)}
  {@const hi = peak(values)}
  <div class="spark" style:--n={values.length}>
    {#each values as v, i (i)}
      <div class="col" title="{labels[i]}: {msgs(v)}">
        <i class:hi={i === hi} style:height="{Math.max(2, (100 * v) / max)}%"></i>
        <span>{i % every === 0 ? labels[i] : ""}</span>
      </div>
    {/each}
  </div>
{/snippet}

{#snippet run(label: string, r: Run | null)}
  <div class="run">
    <div class="label">{label}</div>
    <div class="value">{r ? daysN(r.days) : "–"}</div>
    {#if r}<div class="sub num">{br(r.from)} a {br(r.to)}</div>{/if}
  </div>
{/snippet}

<div class="page" style:--accent={accent}>
  <div class="card fill s4">
    <div class="card-head">
      <h3>Dias de pico</h3>
      <Segmented size="sm" items={METRICS.map((_, i) => ({ value: i }))} value={metric} onchange={(i) => (metric = i)}>
        {#snippet label(i)}{METRICS[i].label}{/snippet}
      </Segmented>
    </div>
    {#if top.length}
      <ol class="grow">
        {#each top as d, i (d.day)}
          <li class:first={i === 0}>
            <span class="rk num">{i + 1}</span>
            <span class="day num">{br(d.day)}</span>
            <span class="wd">{weekdayName(d.day)}</span>
            <span class="bar"><i style:width="{(100 * d.value) / top[0].value}%"></i></span>
            <span class="v num">{m.fmt(d.value)}</span>
          </li>
        {/each}
      </ol>
    {:else}
      <p class="empty">Nenhum dia com {m.label.toLowerCase()} no período.</p>
    {/if}
  </div>

  <div class="stack s2">
    <Stat label="Horário preferido" value="{peak(byHour)}h às {peak(byHour) + 1}h" sub={msgs(byHour[peak(byHour)])}>
      {@render spark(byHour, [...Array(24).keys()].map((x) => `${x}h`), 6)}
    </Stat>
    <Stat label="Dia preferido" value={WEEKDAYS[peak(byWeekday)]} sub={msgs(byWeekday[peak(byWeekday)])}>
      {@render spark(byWeekday, WEEKDAYS_SHORT, 1)}
    </Stat>
    <div class="card runs">
      {@render run("Dias seguidos com mensagens", s.longest_streak)}
      {@render run("Maior intervalo sem mensagens", s.longest_gap)}
    </div>
  </div>

  <div class="card s3">
    <div class="card-head"><h3>Emojis mais usados</h3></div>
    {#if s.extras_pending}
      <p class="none">Calculando…</p>
    {:else if h.emojis.length}
      <ol class="used">
        {#each h.emojis as [e, n], i (e)}
          <li><span class="rk num">{i + 1}</span><span class="emoji">{e}</span><span class="v num">{times(n)}</span></li>
        {/each}
      </ol>
    {:else}
      <p class="none">Nenhum emoji no período.</p>
    {/if}
  </div>
  <div class="card s3">
    <div class="card-head"><h3>Figurinhas mais usadas</h3></div>
    {#if s.extras_pending}
      <p class="none">Calculando…</p>
    {:else if h.stickers.length}
      <ol class="used">
        {#each h.stickers as [id, n], i (id)}
          <li><span class="rk num">{i + 1}</span><Sticker {id} /><span class="v num">{times(n)}</span></li>
        {/each}
      </ol>
    {:else}
      <p class="none">Nenhuma figurinha no período.</p>
    {/if}
  </div>
</div>

<style>
  ol { list-style: none; margin: 0; padding: 0; display: grid; grid-auto-rows: minmax(38px, 1fr); }
  li {
    display: grid; grid-template-columns: 24px 88px 64px minmax(0, 1fr) 120px; align-items: center; gap: 12px;
    font-size: 13px; border-top: 1px solid var(--grid);
  }
  .rk { color: var(--text-3); text-align: right; }
  .wd { color: var(--text-3); }
  .v { text-align: right; color: var(--text-2); }
  .bar { height: 8px; border-radius: 4px; background: var(--active); overflow: hidden; }
  .bar i { display: block; height: 100%; border-radius: 4px; background: var(--text-3); transition: width 450ms var(--ease); }
  .first { font-size: 15px; }
  .first .day, .first .v { color: var(--text); font-weight: 600; }
  .first .bar i { background: var(--accent); }

  .spark {
    height: 84px; margin-top: 12px; display: grid; grid-template-columns: repeat(var(--n), minmax(0, 1fr)); gap: 3px;
  }
  .col { display: flex; flex-direction: column; justify-content: flex-end; min-height: 0; }
  .col i { display: block; border-radius: 2px; background: var(--active); transition: height 450ms var(--ease); }
  .col i.hi { background: var(--accent); }
  .col span { height: 16px; padding-top: 4px; font-size: 10.5px; color: var(--text-3); white-space: nowrap; overflow: visible; }

  .used { grid-template-columns: repeat(3, minmax(0, 1fr)); grid-auto-rows: auto; gap: 16px; }
  .used li { grid-template-columns: auto; justify-items: center; gap: 8px; border-top: 0; padding: 4px 0; }
  .used .emoji { font-size: 44px; line-height: 56px; height: 56px; }
  .used .rk { text-align: center; }
  .none { font-size: 13px; color: var(--text-2); }

  .runs { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 16px; }
  .run .value { font-size: 18px; font-weight: 600; letter-spacing: -0.02em; margin-top: 2px; }
  .run .sub { font-size: 12px; }
</style>
