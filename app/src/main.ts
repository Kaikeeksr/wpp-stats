import "./fonts.css";
import "./app.css";
import flags from "country-flag-emoji-polyfill/dist/TwemojiCountryFlags.woff2?url";
import { mount } from "svelte";
import App from "./App.svelte";

document.fonts.add(
  new FontFace("Twemoji Country Flags", `url(${flags})`, {
    unicodeRange: "U+1F1E6-1F1FF, U+1F3F4, U+E0062-E0063, U+E0065, U+E0067, U+E006C, U+E006E, U+E0073-E0074, U+E0077, U+E007F",
  }),
);

mount(App, { target: document.getElementById("app")! });
