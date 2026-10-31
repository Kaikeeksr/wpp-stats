<script lang="ts">
  import DataTable, { type Row } from "../components/DataTable.svelte";
  import { FILE_KINDS, K, type Stats } from "../lib/api";
  import { fmtDur, fmtInt, fmtSize, pct } from "../lib/fmt";
  import { KIND_LABELS } from "../lib/labels";
  import { sum } from "../lib/people";

  let { s }: { s: Stats } = $props();

  const total = $derived(sum(s.total.media_bytes));
  const sizes = $derived(
    FILE_KINDS.map((k) => ({ k, bytes: s.total.media_bytes[k] }))
      .filter((x) => x.bytes > 0)
      .sort((a, b) => b.bytes - a.bytes),
  );

  const has = (k: number) => s.total.count[k] > 0;
  const rows = $derived.by((): Row[] => {
    const files: Row[] = [K.Foto, K.Video, K.Figurinha, K.Gif, K.Documento, K.Audio].filter(has).map((k) => ({
      label: KIND_LABELS[k],
      cell: (t) => fmtInt(t.count[k]),
      sec: (t) => (t.count[k] ? fmtSize(t.media_bytes[k]) : ""),
    }));
    const once: Row[] = ([[K.FotoUnica, "Fotos"], [K.VideoUnico, "Vídeos"], [K.AudioUnico, "Áudios"]] as const)
      .filter(([k]) => has(k))
      .map(([k, label]) => ({ label, cell: (t) => fmtInt(t.count[k]) }));
    const other: Row[] = [];
    if (has(K.Video)) other.push({ label: "Tempo de vídeo", cell: (t) => fmtDur(t.dur_ms[K.Video]) });
    if (s.total.captioned)
      other.push({ label: "Mídias com legenda", hint: "Cada legenda também conta como uma mensagem de texto.", cell: (t) => fmtInt(t.captioned) });
    if (has(K.Outros))
      other.push({ label: "Não reconhecidas", hint: "Linhas do WhatsApp em formato ainda não suportado.", cell: (t) => fmtInt(t.count[K.Outros]) });
    return [
      { section: "Arquivos · quantidade e tamanho" },
      ...files,
      { label: "Total", cell: (t) => fmtInt(sum(FILE_KINDS.map((k) => t.count[k]))), sec: (t) => fmtSize(sum(t.media_bytes)) },
      ...(once.length ? [{ section: "Visualização única · sem arquivo" }, ...once] : []),
      ...(other.length ? [{ section: "Outros" }, ...other] : []),
    ];
  });
</script>

<div class="page">
  <div class="card s4">
    <div class="card-head"><h3>Mídias</h3></div>
    <DataTable {s} {rows} />
  </div>
  <div class="card s2">
    <div class="card-head"><h3>Espaço por tipo</h3><span class="label num">{fmtSize(total)}</span></div>
    <ul>
      {#each sizes as x (x.k)}
        <li>
          <div class="row"><span>{KIND_LABELS[x.k]}</span><span class="num">{fmtSize(x.bytes)}<span class="pct">{pct(x.bytes, total)}</span></span></div>
          <div class="bar"><i style:width="{(100 * x.bytes) / sizes[0].bytes}%"></i></div>
        </li>
      {/each}
    </ul>
  </div>
</div>

<style>
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 14px; }
  .row { display: flex; justify-content: space-between; gap: 12px; font-size: 12.5px; color: var(--text-2); margin-bottom: 6px; }
  .row .num { color: var(--text); }
  .pct { display: inline-block; min-width: 40px; text-align: right; color: var(--text-3); font-size: 12px; }
  .bar { height: 6px; border-radius: 3px; background: var(--active); overflow: hidden; }
  .bar i { display: block; height: 100%; border-radius: 3px; background: var(--text-3); transition: width 500ms var(--ease); }
</style>
