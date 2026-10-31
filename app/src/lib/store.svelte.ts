import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { api, type Config, type FolderState, type Opened, type Stats, type Zones } from "./api";
import { dayNum, folderName, toDay, type Day } from "./fmt";
import { setPeople } from "./people";
import { setTheme } from "./theme.svelte";

export type Screen = "home" | "processing" | "dashboard";

export const TABS = ["Visão geral", "Mensagens", "Áudios", "Mídias", "Horários", "Destaques"];

export const app = $state({
  screen: "home" as Screen,
  config: {} as Config,
  zones: { system: "UTC", zones: [] } as Zones,
  opened: null as Opened | null,
  view: { from: null, to: null, zone: "UTC", person: null, tab: 0 } as FolderState,
  stats: null as Stats | null,
  progress: 0,
  loading: "",
  error: "",
  toast: null as Toast | null,
});

export interface Toast {
  text: string;
  bad?: boolean;
  action?: { label: string; run: () => void };
}

let saveTimer = 0;
export function saveConfig() {
  clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => api.saveConfig($state.snapshot(app.config)), 300);
}

const query = () => ({ from: app.view.from, to: app.view.to, person: app.view.person });

function persist() {
  if (!app.opened) return;
  app.config.folders ??= {};
  app.config.folders[app.opened.path] = { ...app.view };
  saveConfig();
}

let toastTimer = 0;
export function toast(text: string, bad = false, action?: Toast["action"], ms = 2800) {
  clearTimeout(toastTimer);
  app.toast = { text, bad, action };
  toastTimer = window.setTimeout(() => (app.toast = null), ms);
}

export function dismissToast() {
  clearTimeout(toastTimer);
  app.toast = null;
}

const MAX_RECENTS = 8;

function setRecents(list: string[]) {
  const favs = app.config.favorites ?? [];
  app.config.recent_folders = list.filter((p, i) => favs.includes(p) || list.slice(0, i).filter((q) => !favs.includes(q)).length < MAX_RECENTS);
  saveConfig();
}

export function toggleFavorite(path: string) {
  const favs = app.config.favorites ?? [];
  app.config.favorites = favs.includes(path) ? favs.filter((p) => p !== path) : [...favs, path];
  saveConfig();
}

export function removeRecent(path: string, name: string) {
  const before = [...(app.config.recent_folders ?? [])];
  setRecents(before.filter((p) => p !== path));
  toast(`${name} removido dos recentes`, false, { label: "Desfazer", run: () => setRecents(before) }, 5000);
}

export function clearRecents() {
  app.config.recent_folders = [];
  app.config.recent_info = {};
  app.config.favorites = [];
  app.config.folders = {};
  saveConfig();
}

let seq = 0;
export async function refresh() {
  const mine = ++seq;
  try {
    const s = await api.stats(query(), app.view.zone);
    if (mine === seq) app.stats = s;
  } catch (e) {
    toast(String(e), true);
  }
  persist();
}

let busy = false;
export async function openFolder(path: string) {
  if (busy) return;
  busy = true;
  app.error = "";
  app.progress = 0;
  app.loading = path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? path;
  app.screen = "processing";
  try {
    const o = await api.open(path);
    setPeople(o.messages);
    app.config.recent_info = { ...app.config.recent_info, [o.path]: { messages: o.messages.reduce((a, b) => a + b, 0), opened: Date.now() } };
    setRecents([o.path, ...(app.config.recent_folders ?? []).filter((p) => p !== o.path)]);
    const view: FolderState = { from: null, to: null, zone: o.origin, person: null, ...app.config.folders?.[o.path], tab: 0 };
    if (view.person !== null && view.person >= o.persons.length) view.person = null;
    app.view = view;
    app.opened = o;
    app.stats = await api.stats(query(), view.zone).catch(() => {
      app.view.zone = o.origin;
      return api.stats(query(), o.origin);
    });
    persist();
    app.screen = "dashboard";
  } catch (e) {
    const msg = String(e);
    app.error = msg.includes("cancelada") ? "" : msg.charAt(0).toUpperCase() + msg.slice(1);
    app.screen = app.opened && app.stats ? "dashboard" : "home";
    if (app.error && app.screen === "dashboard") toast(app.error, true);
  } finally {
    busy = false;
  }
}

export async function pickFolder() {
  const p = await openDialog({ directory: true, title: "Pasta da conversa exportada" });
  if (typeof p === "string") openFolder(p);
}

export async function pickZip() {
  const p = await openDialog({
    title: "Conversa exportada (.zip ou o .txt da pasta extraída)",
    filters: [{ name: "Conversa do WhatsApp", extensions: ["zip", "txt"] }],
  });
  if (typeof p === "string") openFolder(/\.txt$/i.test(p) ? p.replace(/[\\/][^\\/]+$/, "") : p);
}

export function goHome() {
  app.error = "";
  app.screen = "home";
}

export function setTab(i: number) {
  if (i === app.view.tab) return;
  app.view.tab = i;
  persist();
}

export function setPerson(p: number | null) {
  app.view.person = p;
  refresh();
}

export function setZone(z: string) {
  app.view.zone = z;
  if (z !== app.zones.system && !app.zones.zones.some(([n]) => n === z)) {
    app.config.recent_zones = [z, ...(app.config.recent_zones ?? []).filter((x) => x !== z)].slice(0, 5);
  }
  refresh();
}

export const span = (): [Day, Day] => [app.opened!.first!, app.opened!.last!];

const back = (days: number) => (): [Day | null, Day | null] => {
  const [first, last] = span();
  return [toDay(Math.max(dayNum(first), dayNum(last) - days + 1)), last];
};

export const SHORTCUTS: [string, () => [Day | null, Day | null]][] = [
  ["Último mês", back(30)],
  ["3 meses", back(91)],
  ["Ano", back(365)],
  ["Tudo", () => [null, null]],
];

export function activeShortcut(): number {
  if (!app.opened?.first) return -1;
  const [first, last] = span();
  const cur = JSON.stringify([app.view.from, app.view.to]);
  let hit = -1;
  SHORTCUTS.forEach(([, f], i) => {
    const r = f();
    if (JSON.stringify(r[0] === first && r[1] === last ? [null, null] : r) === cur) hit = i;
  });
  return hit;
}

export function setPeriod(from: Day | null, to: Day | null) {
  if (!app.opened?.first) return;
  const [first, last] = span();
  const clamp = (d: Day | null) => (d === null ? null : toDay(Math.min(dayNum(last), Math.max(dayNum(first), dayNum(d)))));
  from = clamp(from);
  to = clamp(to);
  if (from && to && from > to) [from, to] = [to, from];
  if ((from ?? first) === first && (to ?? last) === last) from = to = null;
  app.view.from = from;
  app.view.to = to;
  refresh();
}

export function setThemePref(pref: Config["theme"] & string) {
  app.config.theme = pref;
  setTheme(pref);
  saveConfig();
}

export function setCache(on: boolean) {
  app.config.cache_enabled = on;
  saveConfig();
}

export async function exportAs(format: "csv" | "json" | "pdf" | "pdf-anonimo") {
  const base = format === "pdf-anonimo" ? "relatorio-anonimo" : folderName(app.opened!.path);
  const ext = format === "pdf-anonimo" ? "pdf" : format;
  const dest =
    format === "csv"
      ? await openDialog({ directory: true, title: "Pasta para os arquivos CSV" })
      : await saveDialog({ defaultPath: `${base}.${ext}`, filters: [{ name: ext.toUpperCase(), extensions: [ext] }] });
  if (!dest) return;
  try {
    const files = await api.export(format, query(), app.view.zone, dest);
    toast(files.length > 1 ? `${files.length} arquivos exportados` : "Relatório exportado");
  } catch (e) {
    toast(`Falha na exportação: ${e}`, true);
  }
}

export async function start() {
  const [cfg, zones] = await Promise.all([api.loadConfig(), api.zones()]);
  app.config = cfg;
  app.zones = zones;
  setTheme(cfg.theme ?? "system");
}
