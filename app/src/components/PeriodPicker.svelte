<script lang="ts">
  import { CalendarDays, ChevronLeft, ChevronRight } from "lucide";
  import Icon from "./Icon.svelte";
  import Popover from "./Popover.svelte";
  import { br, fromBr, MONTHS, toDay, type Day } from "../lib/fmt";
  import { app, setPeriod, span } from "../lib/store.svelte";

  let open = $state(false);
  const range = $derived.by((): [Day, Day] => {
    const [first, last] = span();
    return [app.view.from ?? first, app.view.to ?? last];
  });

  let y = $state(0);
  let m = $state(0);
  let pending = $state<Day | null>(null);
  let hover = $state<Day | null>(null);
  $effect(() => {
    if (!open) return;
    const d = range[1];
    y = +d.slice(0, 4);
    m = +d.slice(5, 7) - 1;
    pending = null;
  });

  const step = (k: number) => {
    m += k;
    if (m < 0) [m, y] = [11, y - 1];
    if (m > 11) [m, y] = [0, y + 1];
  };

  const cells = $derived.by(() => {
    const start = Math.round(Date.UTC(y, m, 1) / 86_400_000);
    const lead = (start + 3) % 7;
    const n = new Date(Date.UTC(y, m + 1, 0)).getUTCDate();
    return [...Array(lead).fill(null), ...Array.from({ length: n }, (_, i) => toDay(start + i))] as (Day | null)[];
  });

  const shown = $derived.by((): [Day, Day] => {
    if (!pending) return range;
    const b = hover ?? pending;
    return pending < b ? [pending, b] : [b, pending];
  });

  function pick(d: Day) {
    if (!pending) {
      pending = d;
      return;
    }
    setPeriod(pending, d);
    pending = null;
    open = false;
  }

  function mask(e: Event & { currentTarget: HTMLInputElement }) {
    const d = e.currentTarget.value.replace(/\D/g, "").slice(0, 8);
    e.currentTarget.value = [d.slice(0, 2), d.slice(2, 4), d.slice(4)].filter(Boolean).join("/");
    e.currentTarget.classList.remove("bad");
  }

  function commit(which: "from" | "to", el: HTMLInputElement) {
    const d = fromBr(el.value);
    if (!d) {
      el.classList.add("bad");
      return;
    }
    which === "from" ? setPeriod(d, app.view.to) : setPeriod(app.view.from, d);
  }

  const [first, last] = $derived(span());
</script>

<Popover bind:open align="start">
  {#snippet trigger(toggle, isOpen)}
    <button class="trigger" class:on={isOpen} onclick={toggle}>
      <Icon icon={CalendarDays} size={15} />
      <span class="num">{br(range[0])} – {br(range[1])}</span>
    </button>
  {/snippet}
  {#snippet children()}
    <div class="picker">
      <div class="fields">
        {#each [["from", "De", range[0]], ["to", "Até", range[1]]] as const as [id, label, val] (id + val)}
          <label>
            <span>{label}</span>
            <input
              class="num"
              inputmode="numeric"
              maxlength="10"
              placeholder="dd/mm/aaaa"
              value={br(val)}
              oninput={mask}
              onchange={(e) => commit(id, e.currentTarget)}
              onkeydown={(e) => e.key === "Enter" && commit(id, e.currentTarget)} />
          </label>
        {/each}
      </div>
      <div class="head">
        <button class="icon-btn" onclick={() => step(-1)} aria-label="Mês anterior"><Icon icon={ChevronLeft} size={15} /></button>
        <span>{MONTHS[m]} {y}</span>
        <button class="icon-btn" onclick={() => step(1)} aria-label="Próximo mês"><Icon icon={ChevronRight} size={15} /></button>
      </div>
      <div class="cal" role="grid" tabindex="-1" onmouseleave={() => (hover = null)}>
        {#each ["S", "T", "Q", "Q", "S", "S", "D"] as d, i (i)}<span class="wd">{d}</span>{/each}
        {#each cells as d, i (i)}
          {#if d}
            {@const inside = d >= shown[0] && d <= shown[1]}
            <button
              class="day num"
              class:inside
              class:edge={d === shown[0] || d === shown[1]}
              class:lo={d === shown[0]}
              class:hi={d === shown[1]}
              disabled={d < first || d > last}
              onmouseenter={() => (hover = d)}
              onclick={() => pick(d)}>{+d.slice(8)}</button>
          {:else}
            <span></span>
          {/if}
        {/each}
      </div>
      <p class="tip">{pending ? "Escolha o último dia" : "Clique no primeiro e no último dia"}</p>
    </div>
  {/snippet}
</Popover>

<style>
  .trigger {
    display: inline-flex; align-items: center; gap: 8px; height: 30px; padding: 0 12px; border-radius: var(--r-sm);
    font-weight: 500; box-shadow: inset 0 0 0 1px var(--border-2); transition: background 150ms var(--ease);
  }
  .trigger :global(svg) { color: var(--text-2); }
  .trigger:hover, .trigger.on { background: var(--hover); }
  .picker { width: 272px; padding: 6px; }
  .fields { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin-bottom: 12px; }
  label span { display: block; font-size: 11.5px; color: var(--text-3); margin: 0 0 4px 2px; }
  input {
    width: 100%; height: 30px; padding: 0 10px; border: 0; border-radius: 7px; background: var(--active); outline: none;
    transition: box-shadow 150ms var(--ease);
  }
  input:focus { box-shadow: inset 0 0 0 1.5px var(--ink); }
  input:global(.bad) { box-shadow: inset 0 0 0 1.5px var(--bad); }
  .head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px; font-weight: 600; text-transform: capitalize; }
  .cal { display: grid; grid-template-columns: repeat(7, 1fr); row-gap: 2px; }
  .wd { text-align: center; font-size: 11px; color: var(--text-3); padding-bottom: 4px; }
  .day { height: 32px; font-size: 12.5px; position: relative; transition: background 100ms; }
  .day:not(:disabled):hover { background: var(--hover); border-radius: 7px; }
  .day.inside { background: var(--active); border-radius: 0; }
  .day.lo { border-radius: 7px 0 0 7px; }
  .day.hi { border-radius: 0 7px 7px 0; }
  .day.lo.hi { border-radius: 7px; }
  .day.edge { background: var(--ink); color: var(--on-ink); font-weight: 600; }
  .day:disabled { opacity: 0.3; }
  .tip { font-size: 11.5px; color: var(--text-3); text-align: center; margin-top: 8px; }
</style>
