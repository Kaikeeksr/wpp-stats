import type { DayValue } from "./api";
import { br, weekday, WEEKDAYS, type Day } from "./fmt";

export const KIND_LABELS = [
  "Texto", "Áudio", "Foto", "Vídeo", "Figurinha", "GIF", "Documento",
  "Foto de visualização única", "Vídeo de visualização única", "Áudio de visualização única", "Outros",
];

export const PER_DAY = "Total do período dividido pelos dias corridos do intervalo selecionado.";

export const weekdayName = (d: Day) => WEEKDAYS[weekday(d)].toLowerCase();

export const topDay = (l: DayValue[], f: (v: number) => string) =>
  l[0]?.value ? { value: br(l[0].day), sub: `${weekdayName(l[0].day)} · ${f(l[0].value)}` } : { value: "–", sub: "" };
