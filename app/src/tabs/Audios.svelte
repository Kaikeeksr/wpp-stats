<script lang="ts">
  import Chart from "../components/Chart.svelte";
  import DataTable from "../components/DataTable.svelte";
  import Stat from "../components/Stat.svelte";
  import { K, type Stats, type Totals } from "../lib/api";
  import { audiosN, fmtDec, fmtDur, fmtInt, fmtSecs, fmtSize } from "../lib/fmt";
  import { PER_DAY, topDay } from "../lib/labels";
  import { bars, HOURS } from "../lib/options";

  let { s }: { s: Stats } = $props();
  const a = (t: Totals) => t.count[K.Audio];
</script>

<div class="page">
  <div class="card s4">
    <div class="card-head"><h3>Áudios</h3></div>
    <DataTable
      {s}
      rows={[
        { label: "Quantidade", cell: (t) => fmtInt(a(t)) },
        { label: "Tempo total", cell: (t) => fmtDur(t.dur_ms[K.Audio]) },
        { label: "Duração média", cell: (t) => (a(t) ? fmtSecs(t.dur_ms[K.Audio] / 1000 / a(t)) : "–") },
        { label: "Minutos por dia", hint: PER_DAY, cell: (t) => fmtDec(t.audio_min_per_day) },
        { label: "Tamanho", cell: (t) => fmtSize(t.media_bytes[K.Audio]) },
      ]} />
  </div>
  <div class="stack s2">
    <Stat label="Dia com mais tempo de áudio" {...topDay(s.highlights.audio_ms, fmtDur)} />
    <Stat label="Dia com mais áudios" {...topDay(s.highlights.audios, audiosN)} />
  </div>

  <div class="card s6">
    <div class="card-head"><h3>Áudios por hora do dia</h3></div>
    <Chart height={220} option={bars(s, HOURS, (p) => s.audio_by_hour[p], audiosN)} />
  </div>
</div>
