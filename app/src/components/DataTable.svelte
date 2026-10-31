<script lang="ts" module>
  import type { Totals } from "../lib/api";

  export type Row =
    | { section: string }
    | { label: string; hint?: string; cell: (t: Totals, p: number | null) => string; sec?: (t: Totals) => string };
</script>

<script lang="ts">
  import Hint from "./Hint.svelte";
  import Who from "./Who.svelte";
  import type { Stats } from "../lib/api";
  import { colorOf, isGroup, shown } from "../lib/people";

  let { s, rows, total = true }: { s: Stats; rows: Row[]; total?: boolean } = $props();

  type Line = Exclude<Row, { section: string }>;
  const ps = $derived(shown(s));
  const withTotal = $derived(total && ps.length > 1);
  const transposed = $derived(isGroup(s) && ps.length > 2);
  const blocks = $derived.by(() => {
    const out: { title: string; rows: Line[] }[] = [{ title: "", rows: [] }];
    for (const r of rows) {
      if ("section" in r) out.push({ title: r.section, rows: [] });
      else out[out.length - 1].rows.push(r);
    }
    return out.filter((b) => b.rows.length);
  });
</script>

{#snippet value(r: Line, t: Totals, p: number | null)}
  {r.cell(t, p)}{#if r.sec}<span class="sec">{r.sec(t)}</span>{/if}
{/snippet}

{#if transposed}
  {#each blocks as b (b.title)}
    {#if b.title}<div class="tsec">{b.title}</div>{/if}
    <div class="scroll-x">
      <table class="data">
        <thead><tr><th></th>{#each b.rows as r (r.label)}<th>{r.label}{#if r.hint}<Hint text={r.hint} />{/if}</th>{/each}</tr></thead>
        <tbody>
          {#each ps as p (p)}
            <tr><td><Who name={s.persons[p]} color={colorOf(p)} /></td>{#each b.rows as r (r.label)}<td>{@render value(r, s.per_person[p], p)}</td>{/each}</tr>
          {/each}
          <tr class="total"><td>Total</td>{#each b.rows as r (r.label)}<td>{@render value(r, s.total, null)}</td>{/each}</tr>
        </tbody>
      </table>
    </div>
  {/each}
{:else}
  <table class="data">
    <thead>
      <tr>
        <th></th>
        {#each ps as p (p)}<th><Who name={s.persons[p]} color={colorOf(p)} /></th>{/each}
        {#if withTotal}<th class="tot">Total</th>{/if}
      </tr>
    </thead>
    <tbody>
      {#each rows as r, i (i)}
        {#if "section" in r}
          <tr class="section"><td colspan={ps.length + 2}>{r.section}</td></tr>
        {:else}
          <tr>
            <td>{r.label}{#if r.hint}<Hint text={r.hint} />{/if}</td>
            {#each ps as p (p)}<td>{@render value(r, s.per_person[p], p)}</td>{/each}
            {#if withTotal}<td class="tot">{@render value(r, s.total, null)}</td>{/if}
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .tsec { font-size: 12px; color: var(--text-3); font-weight: 500; margin: 20px 0 8px; }
  th :global(.who) { justify-content: flex-end; max-width: 160px; }
</style>
