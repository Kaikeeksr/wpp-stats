import type { ThemePref } from "./api";

const mq = matchMedia("(prefers-color-scheme: dark)");

export const theme = $state({ pref: "system" as ThemePref, dark: mq.matches });

export const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

export function setTheme(pref: ThemePref) {
  theme.pref = pref;
  const dark = pref === "dark" || (pref === "system" && mq.matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  theme.dark = dark;
}

mq.addEventListener("change", () => setTheme(theme.pref));

export const cssVar = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();

const PERSON_LIGHT = ["#4A8BCB", "#D08A45", "#2E9E7A", "#B9659A", "#B89A1E", "#7B68CC", "#C95A40", "#3E9C9C"];
const PERSON_DARK = ["#6BA8E6", "#E3A25E", "#56C09E", "#D58BB8", "#DDC566", "#A394E8", "#E0836B", "#7CC6C6"];

export const personPalette = () => (theme.dark ? PERSON_DARK : PERSON_LIGHT);
export const totalColor = () => (theme.dark ? "#6B6964" : "#B5B1A9");
