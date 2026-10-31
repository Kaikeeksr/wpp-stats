<script lang="ts">
  import Chart from "../components/Chart.svelte";
  import DataTable from "../components/DataTable.svelte";
  import Hint from "../components/Hint.svelte";
  import type { Stats } from "../lib/api";
  import { fmtDec, fmtInt, msgs, pct, WEEKDAYS_SHORT } from "../lib/fmt";
  import { PER_DAY } from "../lib/labels";
  import { bars, heatmap, HOURS } from "../lib/options";
  import { sum } from "../lib/people";

  let { s }: { s: Stats } = $props();

  const perDay = (n: number, days: number) => (days ? fmtDec(n / days) : "–");
</script>

<div class="page">
  <div class="card s6">
    <div class="card-head">
      <h3>Mensagens por dia da semana e hora</h3>
      <span class="scale">menos<i></i><i></i><i></i><i></i>mais</span>
    </div>
    <Chart height={250} option={heatmap(s.heatmap_total)} />
  </div>
  <div class="card s3">
    <div class="card-head"><h3>Por dia da semana</h3></div>
    <Chart height={210} option={bars(s, WEEKDAYS_SHORT, (p) => s.heatmap[p].map(sum), msgs)} />
  </div>
  <div class="card s3">
    <div class="card-head"><h3>Por hora do dia</h3></div>
    <Chart height={210} option={bars(s, HOURS, (p) => HOURS.map((_, h) => sum(s.heatmap[p].map((row) => row[h]))), msgs)} />
  </div>
  <div class="card s3">
    <div class="card-head"><h3>Fim de semana e dias úteis<Hint text="Fim de semana: sábado e domingo, no fuso de exibição." /></h3></div>
    <DataTable
      {s}
      rows={[
        { label: "Fim de semana", cell: (t) => fmtInt(t.weekend), sec: (t) => pct(t.weekend, t.messages) },
        { label: "Dias úteis", cell: (t) => fmtInt(t.messages - t.weekend), sec: (t) => pct(t.messages - t.weekend, t.messages) },
        { label: "Por dia no fim de semana", hint: "Mensagens do fim de semana divididas pelos sábados e domingos do período.", cell: (t) => perDay(t.weekend, s.weekend_days) },
        { label: "Por dia útil", hint: "Mensagens de segunda a sexta divididas pelos dias úteis do período.", cell: (t) => perDay(t.messages - t.weekend, s.days - s.weekend_days) },
      ]} />
  </div>
  <div class="card s3">
    <div class="card-head"><h3>Madrugada<Hint text="De 0h00 a 4h59, no fuso de exibição." /></h3></div>
    <DataTable
      {s}
      rows={[
        { label: "De 0h a 4h59", cell: (t) => fmtInt(t.night), sec: (t) => pct(t.night, t.messages) },
        { label: "De 5h a 23h59", cell: (t) => fmtInt(t.messages - t.night), sec: (t) => pct(t.messages - t.night, t.messages) },
        { label: "Por dia na madrugada", hint: PER_DAY, cell: (t) => perDay(t.night, s.days) },
      ]} />
  </div>
</div>

<style>
  .scale { display: inline-flex; align-items: center; gap: 3px; font-size: 11.5px; color: var(--text-3); }
  .scale i { width: 10px; height: 10px; border-radius: 3px; background: var(--heat-1); }
  .scale i:nth-of-type(1) { margin-left: 4px; background: var(--heat-0); box-shadow: inset 0 0 0 1px var(--border); }
  .scale i:nth-of-type(2) { opacity: 0.35; }
  .scale i:nth-of-type(3) { opacity: 0.65; }
  .scale i:nth-of-type(4) { margin-right: 4px; }
</style>
