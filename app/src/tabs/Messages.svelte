<script lang="ts">
  import Chart from "../components/Chart.svelte";
  import DataTable from "../components/DataTable.svelte";
  import Hint from "../components/Hint.svelte";
  import Who from "../components/Who.svelte";
  import { K, type Stats, type Totals } from "../lib/api";
  import { fmtDec, fmtInt, fmtSecs, msgs, plural } from "../lib/fmt";
  import { KIND_LABELS, PER_DAY } from "../lib/labels";
  import { bars } from "../lib/options";
  import { colorOf, isGroup, shown } from "../lib/people";

  let { s }: { s: Stats } = $props();

  const kinds = $derived(KIND_LABELS.map((_, k) => k).filter((k) => s.total.count[k] > 0));
  const avgChars = (t: Totals) => (t.count[K.Texto] ? fmtDec(t.text_chars / t.count[K.Texto]) : "–");
  const rsp = $derived(
    "Em X% das vezes a pessoa respondeu em até esse tempo, contado da última mensagem da outra pessoa até a primeira dela, sem limite máximo." +
      (isGroup(s) ? " Em grupo, não indica a quem a pessoa respondeu." : ""),
  );
  const time = (x: number | null) => (x === null ? "–" : fmtSecs(x));
</script>

<div class="page">
  <div class="card s3">
    <div class="card-head"><h3>Tempo de resposta<Hint text={rsp} /></h3></div>
    {#if shown(s).length > 2}
      <table class="data">
          <thead><tr><th></th><th>Em 50% das vezes</th><th>Em 75%</th><th>Em 90%</th><th>Respostas</th></tr></thead>
          <tbody>
            {#each shown(s) as p (p)}
              {@const r = s.response[p]}
              <tr>
                <td><Who name={s.persons[p]} color={colorOf(p)} /></td>
                <td>{time(r.median_s)}</td>
                <td>{time(r.p75_s)}</td>
                <td>{time(r.p90_s)}</td>
                <td class="tot">{fmtInt(r.count)}</td>
              </tr>
            {/each}
          </tbody>
      </table>
    {:else}
      <div class="replies" style:--n={shown(s).length}>
        {#each shown(s) as p (p)}
          {@const r = s.response[p]}
          <div class="reply" style:--c={colorOf(p)}>
            <div>
              <Who name={s.persons[p]} color={colorOf(p)} />
              <p class="lead">Em 50% das vezes responde em até</p>
              <div class="big">{time(r.median_s)}</div>
            </div>
            <div class="qs">
              {#each [[75, r.p75_s], [90, r.p90_s]] as const as [q, v] (q)}
                <div class="q">
                  <span>Em {q}% das vezes, até <b class="num">{time(v)}</b></span>
                  <div class="qbar"><i style:width="{q}%"></i></div>
                </div>
              {/each}
              <p class="n num">{plural(r.count, "resposta", "respostas")}</p>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <div class="card s3">
    <div class="card-head"><h3>Texto</h3></div>
    <DataTable
        {s}
        rows={[
          { label: "Mensagens de texto", cell: (t) => fmtInt(t.count[K.Texto]) },
          { label: "Mensagens por dia", hint: PER_DAY, cell: (t) => fmtDec(t.messages_per_day) },
          { label: "Caracteres", cell: (t) => fmtInt(t.text_chars) },
          { label: "Caracteres por mensagem", cell: avgChars },
          { label: "Bytes (UTF-8)", cell: (t) => fmtInt(t.text_bytes) },
          { label: "Links", hint: "Endereços que começam com http://, https:// ou www., em mensagens e legendas.", cell: (t) => fmtInt(t.links) },
          { label: "Emojis", cell: (t) => (s.extras_pending ? "…" : fmtInt(t.emojis)) },
          { label: "Mensagens apagadas", hint: "Não entram em nenhum outro número.", cell: (t) => fmtInt(t.deleted) },
        ]} />
  </div>

  <div class="card s6">
    <div class="card-head"><h3>Mensagens por tipo</h3></div>
    <Chart
      height={kinds.length * 30 + 40}
      option={bars(s, kinds.map((k) => KIND_LABELS[k]), (p) => kinds.map((k) => s.per_person[p].count[k]), msgs, true)} />
  </div>
</div>

<style>
  .replies { display: grid; grid-template-columns: repeat(var(--n), minmax(0, 1fr)); gap: 24px 32px; }
  .reply { display: grid; gap: 14px; align-content: start; }
  .lead { font-size: 12.5px; color: var(--text-2); margin-top: 10px; }
  .qs { display: grid; gap: 10px; }
  .q { display: grid; gap: 5px; font-size: 12.5px; color: var(--text-2); white-space: nowrap; }
  .q b { color: var(--text); font-weight: 600; }
  .qbar { height: 4px; border-radius: 2px; background: var(--active); overflow: hidden; }
  .qbar i { display: block; height: 100%; border-radius: 2px; background: var(--c); animation: grow 700ms var(--ease) both; transform-origin: left; }
  .n { font-size: 12px; color: var(--text-3); }
  @keyframes grow { from { transform: scaleX(0); } }
</style>
