const int = new Intl.NumberFormat("pt-BR", { maximumFractionDigits: 0 });

export const fmtInt = (n: number) => int.format(n);

export const fmtDec = (x: number, places = 1) =>
  x.toLocaleString("pt-BR", { minimumFractionDigits: places, maximumFractionDigits: places });

export function fmtDur(ms: number): string {
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h === 0 && m === 0) return `${s}s`;
  if (h === 0) return `${m}min`;
  return `${fmtInt(h)}h ${String(m).padStart(2, "0")}min`;
}

export function fmtSecs(x: number): string {
  const s = Math.round(x);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h === 0 && m === 0) return `${s % 60}s`;
  if (h === 0) return `${m}min ${String(s % 60).padStart(2, "0")}s`;
  return `${fmtInt(h)}h ${String(m).padStart(2, "0")}min`;
}

export function fmtSize(b: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let x = b;
  let u = 0;
  while (x >= 1024 && u < units.length - 1) {
    x /= 1024;
    u++;
  }
  return u === 0 ? `${b} B` : `${fmtDec(x)} ${units[u]}`;
}

export type Day = string;

export const br = (d: Day) => `${d.slice(8, 10)}/${d.slice(5, 7)}/${d.slice(0, 4)}`;

export function fromBr(s: string): Day | null {
  const m = /^(\d{2})\/(\d{2})\/(\d{4})$/.exec(s);
  if (!m) return null;
  const d = `${m[3]}-${m[2]}-${m[1]}`;
  return toDay(dayNum(d)) === d ? d : null;
}

export const dayNum = (d: Day) => Math.round(Date.UTC(+d.slice(0, 4), +d.slice(5, 7) - 1, +d.slice(8, 10)) / 86_400_000);

export const toDay = (n: number) => new Date(n * 86_400_000).toISOString().slice(0, 10);

export const WEEKDAYS = ["Segunda", "Terça", "Quarta", "Quinta", "Sexta", "Sábado", "Domingo"];
export const WEEKDAYS_SHORT = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];
export const MONTHS = [
  "janeiro", "fevereiro", "março", "abril", "maio", "junho",
  "julho", "agosto", "setembro", "outubro", "novembro", "dezembro",
];

export const weekday = (d: Day) => (dayNum(d) + 3) % 7;

export const esc = (s: string) =>
  s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);

export const plural = (v: number, one: string, many: string) => `${fmtInt(v)} ${Math.round(v) === 1 ? one : many}`;
export const msgs = (v: number) => plural(v, "mensagem", "mensagens");
export const audiosN = (v: number) => plural(v, "áudio", "áudios");
export const mediaN = (v: number) => plural(v, "mídia", "mídias");
export const daysN = (v: number) => plural(v, "dia", "dias");

export const pct = (part: number, whole: number) => `${fmtDec((100 * part) / Math.max(1, whole), 0)}%`;

export function crumbs(path: string, home: string): string[] {
  let p = path.replace(/[\\/]+$/, "");
  if (home && p.toLowerCase().startsWith(home.toLowerCase())) p = "~" + p.slice(home.length);
  const parts = p.split(/[\\/]+/).filter(Boolean);
  return parts.length > 4 ? [parts[0], "…", ...parts.slice(-2)] : parts;
}

export const folderName = (path: string) =>
  (path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? path).replace(/\.zip$/i, "");
