<script lang="ts">
  import { ChevronDown, Globe } from "lucide";
  import Icon from "./Icon.svelte";
  import PeriodPicker from "./PeriodPicker.svelte";
  import Popover from "./Popover.svelte";
  import Segmented from "./Segmented.svelte";
  import { daysN } from "../lib/fmt";
  import { byRank, colorOf } from "../lib/people";
  import { activeShortcut, app, setPeriod, setPerson, setZone, SHORTCUTS } from "../lib/store.svelte";

  const o = $derived(app.opened!);
  const group = $derived(o.persons.length > 2);
  const sc = $derived(activeShortcut());

  const zones = $derived.by(() => {
    const labelOf = (z: string) => app.zones.zones.find(([n]) => n === z)?.[1] ?? z;
    const list: [string, string][] = [[app.zones.system, `${labelOf(app.zones.system)} · Windows`]];
    for (const [z, l] of app.zones.zones) list.push([z, l]);
    for (const z of app.config.recent_zones ?? []) list.push([z, z]);
    list.push([app.view.zone, labelOf(app.view.zone)]);
    return list.filter(([z], i) => list.findIndex(([x]) => x === z) === i);
  });
  const zoneLabel = $derived(zones.find(([z]) => z === app.view.zone)?.[1].replace(" · Windows", "") ?? app.view.zone);
</script>

<div class="filters">
  <div class="left">
    <PeriodPicker />
    <span class="days num">{app.stats?.days ? daysN(app.stats.days) : ""}</span>
    <Segmented size="sm" items={SHORTCUTS.map((_, i) => ({ value: i }))} value={sc} onchange={(i) => setPeriod(...SHORTCUTS[i][1]())}>
      {#snippet label(i)}{SHORTCUTS[i][0]}{/snippet}
    </Segmented>
  </div>
  <div class="right">
    {#if group}
      <Popover align="end">
        {#snippet trigger(toggle, open)}
          <button class="btn quiet" class:on={open} onclick={toggle}>
            {app.view.person === null ? `Todos (${o.persons.length})` : o.persons[app.view.person]}
            <Icon icon={ChevronDown} size={14} />
          </button>
        {/snippet}
        {#snippet children(close)}
          <div class="list">
            <button class="item" class:on={app.view.person === null} onclick={() => (close(), setPerson(null))}>Todos</button>
            {#each byRank(o.persons.length) as p (p)}
              <button class="item" class:on={app.view.person === p} onclick={() => (close(), setPerson(p))}>
                <i class="dot" style:background={colorOf(p)}></i>{o.persons[p]}
              </button>
            {/each}
          </div>
        {/snippet}
      </Popover>
    {:else}
      <Segmented size="sm" items={[{ value: -1 }, ...o.persons.map((p, i) => ({ value: i, title: p }))]} value={app.view.person ?? -1} onchange={(v) => setPerson(v < 0 ? null : v)}>
        {#snippet label(v)}
          {#if v < 0}Ambos{:else}<i class="dot" style:background={colorOf(v)}></i><span class="nm">{o.persons[v]}</span>{/if}
        {/snippet}
      </Segmented>
    {/if}
    <Popover align="end">
      {#snippet trigger(toggle, open)}
        <button class="btn quiet" class:on={open} onclick={toggle} title="Fuso de exibição"><Icon icon={Globe} size={15} />{zoneLabel}</button>
      {/snippet}
      {#snippet children(close)}
        <div class="group-label">Fuso de exibição</div>
        {#each zones as [z, l] (z)}
          <button class="item" class:on={z === app.view.zone} onclick={() => (close(), setZone(z))}>{l}</button>
        {/each}
      {/snippet}
    </Popover>
  </div>
</div>

<style>
  .filters { display: flex; align-items: center; justify-content: space-between; gap: 16px; flex-wrap: wrap; }
  .left, .right { display: flex; align-items: center; gap: 12px; }
  .right { gap: 6px; }
  .days { color: var(--text-3); font-size: 12.5px; min-width: 56px; }
  .dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .nm { overflow: hidden; text-overflow: ellipsis; }
  .list { max-height: 360px; overflow-y: auto; }
</style>
