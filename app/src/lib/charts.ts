import * as echarts from "echarts/core";
import { BarChart, HeatmapChart, LineChart, PieChart } from "echarts/charts";
import {
  BrushComponent,
  GridComponent,
  MarkLineComponent,
  MarkPointComponent,
  ToolboxComponent,
  TooltipComponent,
  VisualMapComponent,
} from "echarts/components";
import { SVGRenderer } from "echarts/renderers";
import type { EChartsCoreOption } from "echarts/core";
import { esc, fmtInt } from "./fmt";
import { cssVar, reducedMotion, theme } from "./theme.svelte";

echarts.use([
  BarChart, HeatmapChart, LineChart, PieChart,
  BrushComponent, GridComponent, MarkLineComponent, MarkPointComponent, ToolboxComponent, TooltipComponent, VisualMapComponent,
  SVGRenderer,
]);

export { echarts };
export type Option = EChartsCoreOption;

const FONT = '"Inter Variable", system-ui, sans-serif';

export function ink() {
  void theme.dark;
  return {
    text: cssVar("--text"),
    text2: cssVar("--text-2"),
    text3: cssVar("--text-3"),
    border: cssVar("--border"),
    grid: cssVar("--grid"),
    surface: cssVar("--surface"),
    hover: cssVar("--hover"),
    heat: [cssVar("--heat-0"), cssVar("--heat-1")],
  };
}

export function tooltip() {
  const c = ink();
  return {
    backgroundColor: c.surface,
    borderColor: c.border,
    borderWidth: 1,
    padding: [8, 12],
    textStyle: { color: c.text, fontSize: 12, fontFamily: FONT },
    extraCssText: "border-radius:10px;box-shadow:0 4px 16px rgba(0,0,0,.08);",
    transitionDuration: 0.15,
  };
}

export function axis() {
  const c = ink();
  return {
    axisLine: { show: false },
    axisTick: { show: false },
    axisLabel: { color: c.text2, fontSize: 12, fontFamily: FONT },
    splitLine: { show: false },
  };
}

export function valueAxis() {
  const a = axis();
  return {
    type: "value",
    ...a,
    axisLabel: { ...a.axisLabel, color: ink().text3, fontSize: 11, formatter: (v: number) => fmtInt(v) },
    splitLine: { show: true, lineStyle: { color: ink().grid } },
    splitNumber: 3,
  };
}

export function base(): Option {
  return {
    animation: !reducedMotion,
    animationDuration: 700,
    animationEasing: "cubicOut",
    animationDurationUpdate: 450,
    animationEasingUpdate: "cubicOut",
    textStyle: { fontFamily: FONT },
    tooltip: { trigger: "axis", ...tooltip() },
  };
}

export function tipRows(head: string, rows: { marker: string; name: string; value: string }[], total?: string): string {
  return `<div class="tt-h">${esc(head)}</div>${rows
    .map((r) => `<div class="tt">${r.marker}<span>${esc(r.name)}</span><b>${r.value}</b></div>`)
    .join("")}${total ? `<div class="tt tt-t"><span>Total</span><b>${total}</b></div>` : ""}`;
}

export function brush(): Option {
  return {
    toolbox: { show: false },
    brush: {
      xAxisIndex: 0,
      brushType: "lineX",
      brushMode: "single",
      transformable: false,
      removeOnClick: true,
      brushStyle: { color: "rgba(127,127,127,0.10)", borderColor: "rgba(127,127,127,0.35)", borderWidth: 1 },
      outOfBrush: { colorAlpha: 1 },
    },
  };
}
