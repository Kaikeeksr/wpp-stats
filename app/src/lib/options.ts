import type { Series, Stats } from "./api";
import { axis, ink, tipRows, tooltip, valueAxis, type Option } from "./charts";
import { br, dayNum, fmtInt, msgs, toDay, weekday, WEEKDAYS, type Day } from "./fmt";
import { groups, gvec, isGroup, shown, sum, type Group } from "./people";

export const HOURS = [...Array(24).keys()].map((x) => `${x}h`);

const dayAxis = (labels: Day[]) => {
  const a = axis();
  return { type: "category", data: labels.map(br), boundaryGap: false, ...a, axisLabel: { ...a.axisLabel, hideOverlap: true } };
};

function stackTip(s: Stats, unit: (v: number) => string) {
  return (ps: any[]) =>
    tipRows(
      ps[0]?.axisValue ?? "",
      ps.map((p) => ({ marker: p.marker, name: p.seriesName, value: unit(p.value) })),
      shown(s).length > 1 ? unit(ps.reduce((a, p) => a + p.value, 0)) : undefined,
    );
}

export type Grain = Series["bucket"];

export const GRAIN_LABELS: Record<Grain, string> = { dia: "Dia", semana: "Semana", mes: "Mês" };

export function grains(tl: Series): Grain[] {
  const all: Grain[] = ["dia", "semana", "mes"];
  return all.slice(all.indexOf(tl.bucket)).filter((g) => g !== "dia" || tl.labels.length <= 366);
}

export const defaultGrain = (tl: Series): Grain => (tl.bucket === "dia" && tl.labels.length > 90 ? "semana" : tl.bucket);

export function regroup(tl: Series, grain: Grain): Series {
  if (grain === tl.bucket) return tl;
  const start = (d: Day) => (grain === "mes" ? `${d.slice(0, 8)}01` : toDay(dayNum(d) - weekday(d)));
  const out: Series = { bucket: grain, labels: [], total: [], per_person: tl.per_person.map(() => []) };
  tl.labels.forEach((d, i) => {
    const b = start(d);
    if (out.labels[out.labels.length - 1] !== b) {
      out.labels.push(b);
      out.total.push(0);
      out.per_person.forEach((v) => v.push(0));
    }
    out.total[out.total.length - 1] += tl.total[i];
    out.per_person.forEach((v, p) => (v[v.length - 1] += tl.per_person[p][i]));
  });
  return out;
}

export function trend(series: Series): Option {
  const c = ink();
  const v = series.total;
  const last = v.length - 1;
  const hi = v.indexOf(Math.max(...v));
  const avg = sum(v) / Math.max(1, v.length);
  const edge = (i: number) => (i < v.length * 0.1 ? "left" : i > v.length * 0.9 ? "right" : "center");
  const pt = (i: number, strong: boolean) => ({
    coord: [i, v[i]],
    value: v[i],
    symbol: "circle",
    symbolSize: strong ? 9 : 7,
    itemStyle: { color: c.text, borderColor: c.surface, borderWidth: 2 },
    label: {
      show: true, position: "top", align: edge(i), distance: 8, formatter: () => fmtInt(v[i]),
      color: strong ? c.text : c.text2, fontSize: 12, fontWeight: 600,
    },
  });
  return {
    grid: { left: 44, right: 16, top: 28, bottom: 24 },
    tooltip: { trigger: "axis", ...tooltip(), valueFormatter: msgs },
    xAxis: dayAxis(series.labels),
    yAxis: valueAxis(),
    series: [
      {
        type: "line", name: "Mensagens", data: v, showSymbol: false, smooth: 0.3, smoothMonotone: "x",
        lineStyle: { width: 2, color: c.text, cap: "round", join: "round" },
        itemStyle: { color: c.text },
        areaStyle: { color: c.text, opacity: 0.04 },
        emphasis: { disabled: true },
        markPoint: { silent: true, data: [pt(hi, false), ...(last !== hi ? [pt(last, true)] : [])] },
        markLine: {
          silent: true, symbol: "none", label: { show: false },
          lineStyle: { color: c.text3, type: [4, 4], width: 1 },
          data: [{ yAxis: avg }],
        },
      },
    ],
  };
}

export function cumulative(s: Stats): Option {
  const series = s.progression;
  const gs = groups(s);
  const ends = gs.map((g) => gvec(g, (p) => series.per_person[p]).at(-1) ?? 0);
  const right = 14 + 7.5 * Math.max(...ends.map((x) => fmtInt(x).length), 1);
  const order = [...ends.keys()].sort((a, b) => ends[b] - ends[a]);
  const top = Math.max(1, ...ends);
  const crowded = order.some((p, i) => i > 0 && (ends[order[i - 1]] - ends[p]) / top < 0.08);
  const shift = (g: number) => (crowded ? (order.indexOf(g) - (order.length - 1) / 2) * 14 : 0);
  return {
    grid: { left: 44, right, top: 16, bottom: 24 },
    tooltip: { trigger: "axis", ...tooltip(), formatter: stackTip(s, msgs) },
    xAxis: dayAxis(series.labels),
    yAxis: valueAxis(),
    series: gs.map((g, gi) => ({
      type: "line", name: g.name, data: gvec(g, (p) => series.per_person[p]), showSymbol: false, smooth: 0.2, smoothMonotone: "x",
      lineStyle: { width: 2, color: g.color }, itemStyle: { color: g.color }, areaStyle: { color: g.color, opacity: 0.06 },
      emphasis: { disabled: true },
      endLabel: { show: true, color: g.color, fontSize: 12, fontWeight: 600, formatter: (p: any) => fmtInt(p.value), distance: 6, offset: [0, shift(gi)] },
    })),
  };
}

export function donut(gs: Group[], value: (g: Group) => number): Option {
  return {
    tooltip: { trigger: "item", ...tooltip(), formatter: (p: any) => tipRows(p.name, [{ marker: p.marker, name: "", value: msgs(p.value) }]) },
    series: [{
      type: "pie", radius: ["60%", "90%"], label: { show: false }, emphasis: { scale: false }, padAngle: 2,
      itemStyle: { borderRadius: 4, borderWidth: 0 },
      animationType: "scale", animationEasing: "cubicOut",
      data: gs.map((g) => ({ name: g.name, value: value(g), itemStyle: { color: g.color } })),
    }],
  };
}

export function bars(s: Stats, cats: string[], data: (p: number) => number[], unit: (v: number) => string, horizontal = false): Option {
  const cat = { type: "category", data: cats, ...axis(), inverse: horizontal };
  const val = valueAxis();
  return {
    grid: { left: horizontal ? 190 : 44, right: 24, top: 8, bottom: 24 },
    tooltip: { trigger: "axis", axisPointer: { type: "shadow", shadowStyle: { color: ink().hover } }, ...tooltip(), formatter: stackTip(s, unit) },
    xAxis: horizontal ? val : cat,
    yAxis: horizontal ? cat : val,
    series: groups(s).map((g) => ({
      type: "bar", name: g.name, data: gvec(g, data), barMaxWidth: horizontal ? 10 : 12, barGap: "30%",
      itemStyle: { color: g.color, borderRadius: horizontal ? [0, 3, 3, 0] : [3, 3, 0, 0] },
      emphasis: { disabled: true },
      ...(isGroup(s) ? { stack: "t", itemStyle: { color: g.color } } : {}),
    })),
  };
}

export function heatmap(hm: number[][]): Option {
  const c = ink();
  const max = Math.max(1, ...hm.flat());
  return {
    grid: { left: 44, right: 8, top: 4, bottom: 24 },
    tooltip: { trigger: "item", ...tooltip(), formatter: (p: any) => tipRows(`${WEEKDAYS[p.value[1]]}, ${p.value[0]}h`, [{ marker: "", name: "", value: msgs(p.value[3]) }]) },
    xAxis: { type: "category", data: HOURS, ...axis(), axisLabel: { ...axis().axisLabel, interval: 2 } },
    yAxis: { type: "category", data: ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"], inverse: true, ...axis() },
    visualMap: { show: false, min: 0, max: Math.sqrt(max), dimension: 2, inRange: { color: c.heat } },
    series: [{
      type: "heatmap",
      data: hm.flatMap((row, wd) => row.map((v, h) => [h, wd, Math.sqrt(v), v])),
      itemStyle: { borderColor: c.surface, borderWidth: 3, borderRadius: 5 },
      emphasis: { disabled: true },
    }],
  };
}

export function bucketRange(labels: Day[], end: Day, i0: number, i1: number): [Day, Day] {
  i1 = Math.min(i1, labels.length - 1);
  return [labels[i0], i1 + 1 < labels.length ? toDay(dayNum(labels[i1 + 1]) - 1) : end];
}
