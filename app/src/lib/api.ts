import { invoke } from "@tauri-apps/api/core";
import type { Day } from "./fmt";

export interface Opened {
  path: string;
  home: string;
  persons: string[];
  messages: number[];
  first: Day | null;
  last: Day | null;
  origin: string;
  from_cache: boolean;
}

export interface Zones {
  system: string;
  zones: [string, string][];
}

export type ThemePref = "system" | "light" | "dark";

export interface FolderState {
  from: Day | null;
  to: Day | null;
  zone: string;
  person: number | null;
  tab: number;
}

export interface Config {
  cache_enabled?: boolean;
  theme?: ThemePref;
  recent_folders?: string[];
  recent_info?: Record<string, { messages: number; opened: number }>;
  favorites?: string[];
  guide_open?: boolean;
  phone?: "ios" | "android";
  recent_zones?: string[];
  folders?: Record<string, FolderState>;
}

export interface Totals {
  messages: number;
  count: number[];
  text_bytes: number;
  text_chars: number;
  media_bytes: number[];
  dur_ms: number[];
  captioned: number;
  deleted: number;
  links: number;
  emojis: number;
  weekend: number;
  night: number;
  messages_per_day: number;
  audio_min_per_day: number;
}

export interface Series {
  bucket: "dia" | "semana" | "mes";
  labels: Day[];
  total: number[];
  per_person: number[][];
}

export interface DayValue { day: Day; value: number }
export interface Ranked { key: number; value: number }
export interface Run { from: Day; to: Day; days: number }

export interface Response {
  median_s: number | null;
  p75_s: number | null;
  p90_s: number | null;
  count: number;
}

export interface Stats {
  from: Day | null;
  to: Day | null;
  days: number;
  weekend_days: number;
  persons: string[];
  selected: boolean[];
  per_person: Totals[];
  total: Totals;
  response: Response[];
  timeline: Series;
  progression: Series;
  heatmap: number[][][];
  heatmap_total: number[][];
  audio_by_hour: number[][];
  highlights: {
    messages: DayValue[];
    audios: DayValue[];
    audio_ms: DayValue[];
    media: DayValue[];
    hours: Ranked[];
    weekdays: Ranked[];
    emojis: [string, number][];
    stickers: [number, number][];
  };
  longest_streak: Run | null;
  longest_gap: Run | null;
  extras_pending: boolean;
}

export interface Query {
  from: Day | null;
  to: Day | null;
  person: number | null;
}

export const K = { Texto: 0, Audio: 1, Foto: 2, Video: 3, Figurinha: 4, Gif: 5, Documento: 6, FotoUnica: 7, VideoUnico: 8, AudioUnico: 9, Outros: 10 };

export const FILE_KINDS = [K.Audio, K.Foto, K.Video, K.Figurinha, K.Gif, K.Documento];

export const api = {
  loadConfig: () => invoke<Config>("load_config").catch(() => ({}) as Config),
  saveConfig: (config: Config) => invoke("save_config", { config }).catch(() => {}),
  zones: () => invoke<Zones>("list_timezones"),
  open: (path: string) => invoke<Opened>("open_folder", { path }),
  cancel: () => invoke("cancel"),
  stats: (query: Query, zone: string) => invoke<Stats>("get_stats", { query, zone }),
  sticker: (id: number) => invoke<ArrayBuffer>("sticker", { id }),
  export: (format: string, query: Query, zone: string, dest: string) => invoke<string[]>("export", { format, query, zone, dest }),
  clearCache: () => invoke("clear_cache"),
  initialFolder: () => invoke<string | null>("initial_folder").catch(() => null),
};
