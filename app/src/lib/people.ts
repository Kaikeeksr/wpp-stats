import type { Stats } from "./api";
import { personPalette, totalColor } from "./theme.svelte";

let rank: number[] = [];

export function setPeople(messages: number[]) {
  const order = messages.map((_, p) => p);
  if (messages.length > 2) order.sort((a, b) => messages[b] - messages[a] || a - b);
  rank = [];
  order.forEach((p, i) => (rank[p] = i));
}

const rankOf = (p: number) => rank[p] ?? p;

export const byRank = (n: number) => [...Array(n).keys()].sort((a, b) => rankOf(a) - rankOf(b));

export const colorOf = (p: number) => personPalette()[rankOf(p)] ?? totalColor();

export const isGroup = (s: Stats) => s.persons.length > 2;

export const shown = (s: Stats) => byRank(s.persons.length).filter((p) => s.selected[p]);

export interface Group {
  name: string;
  color: string;
  members: number[];
}

export function groups(s: Stats, limit = 8): Group[] {
  const ps = shown(s);
  const own = ps.length > limit ? limit - 1 : limit;
  const top = ps.slice(0, own).filter((p) => rankOf(p) < personPalette().length);
  const rest = ps.filter((p) => !top.includes(p));
  return [
    ...top.map((p) => ({ name: s.persons[p], color: colorOf(p), members: [p] })),
    ...(rest.length ? [{ name: `Demais (${rest.length})`, color: totalColor(), members: rest }] : []),
  ];
}

export const sum = (v: number[]) => v.reduce((a, b) => a + b, 0);

export const gsum = (g: Pick<Group, "members">, f: (p: number) => number) => sum(g.members.map(f));

export const gvec = (g: Group, f: (p: number) => number[]) => g.members.map(f).reduce((a, v) => a.map((x, i) => x + v[i]));
