<script lang="ts">
  import { ChevronDown, CircleAlert, File, Lock, Upload } from "lucide";
  import Icon from "./Icon.svelte";
  import { api } from "../lib/api";
  import { folderName, msgs } from "../lib/fmt";
  import { app, openFolder, pickFolder, pickZip, removeRecent, saveConfig, toggleFavorite } from "../lib/store.svelte";

  type Phone = "ios" | "android";

  const GUIDE: Record<Phone, [string, string][]> = {
    ios: [
      ["Abra a conversa", " e toque no nome do contato no topo."],
      ["Exportar conversa", ": role até o fim › Exportar conversa › Anexar mídia."],
      ["Leve o .zip ao computador", " por AirDrop, iCloud Drive ou e-mail."],
    ],
    android: [
      ["Abra a conversa", " e toque em ⋮ no canto superior direito."],
      ["Exportar conversa", ": Mais › Exportar conversa › Incluir mídia."],
      ["Leve o .zip ao computador", " por Google Drive, e-mail ou cabo USB."],
    ],
  };

  const FIRST_GUIDE: Record<Phone, [string, string][]> = {
    ios: [
      ["Abra a conversa", "Toque no nome do contato no topo."],
      ["Exporte a conversa", "Role até o fim › Exportar conversa › Anexar mídia."],
      ["Leve o .zip ao computador", "Por AirDrop, iCloud Drive ou e-mail, e arraste ao lado."],
    ],
    android: [
      ["Abra a conversa", "Toque em ⋮ no canto superior direito."],
      ["Exporte a conversa", "Mais › Exportar conversa › Incluir mídia."],
      ["Leve o .zip ao computador", "Por Google Drive, e-mail ou cabo USB, e arraste ao lado."],
    ],
  };

  const favs = $derived(app.config.favorites ?? []);
  const recents = $derived.by(() => {
    const all = app.config.recent_folders ?? [];
    return [...all.filter((p) => favs.includes(p)), ...all.filter((p) => !favs.includes(p))];
  });
  const firstUse = $derived(!recents.length && !Object.keys(app.config.recent_info ?? {}).length);
  const phone = $derived<Phone>(app.config.phone ?? "ios");
  const guideOpen = $derived(app.config.guide_open ?? true);
  const notChat = $derived(/conversa exportada|zip/i.test(app.error));

  function setPhone(p: Phone) {
    app.config.phone = p;
    saveConfig();
  }

  function toggleGuide() {
    app.config.guide_open = !guideOpen;
    saveConfig();
  }

  const contact = (p: string) =>
    folderName(p).replace(/^(WhatsApp Chat|Conversa do WhatsApp)(\s+(with|com))?\s*[-–]?\s*/i, "") || folderName(p);

  const rtf = new Intl.RelativeTimeFormat("pt-BR", { numeric: "auto" });
  const UNITS = [["year", 31_536_000], ["month", 2_592_000], ["week", 604_800], ["day", 86_400], ["hour", 3600], ["minute", 60]] as const;
  function ago(ms: number) {
    const s = (ms - Date.now()) / 1000;
    const hit = UNITS.find(([, n]) => Math.abs(s) >= n);
    return hit ? rtf.format(Math.round(s / hit[1]), hit[0]) : "agora";
  }

  function meta(p: string) {
    const info = app.config.recent_info?.[p];
    return info ? `${msgs(info.messages)} · aberto ${ago(info.opened)}` : folderName(p);
  }
</script>

{#snippet phones()}
  <div class="seg" role="group" aria-label="Sistema do celular">
    {#each [["ios", "iOS"], ["android", "Android"]] as const as [v, l] (v)}
      <button class:on={phone === v} aria-pressed={phone === v} onclick={() => setPhone(v)}>{l}</button>
    {/each}
  </div>
{/snippet}

<main class="home">
  <div class="grid">
    <section class="col">
      <h1>Estatísticas de uma conversa do WhatsApp</h1>

      {#if app.screen === "processing"}
        <div class="drop busy">
          <span class="tile"><Icon icon={File} size={24} stroke={1.8} /></span>
          <div class="txt">
            <p class="title">{app.loading}</p>
            <p class="body2">Lendo mensagens…</p>
          </div>
          <div class="bar" role="progressbar" aria-label="Progresso da análise" aria-valuemin={0} aria-valuemax={100} aria-valuenow={app.progress}>
            <i style:transform="scaleX({app.progress / 100})"></i>
          </div>
          <button class="btn-2" onclick={() => api.cancel()}>Cancelar</button>
        </div>
      {:else if app.error}
        <div class="drop bad" role="alert">
          <span class="tile"><Icon icon={CircleAlert} size={24} stroke={1.8} /></span>
          <div class="txt">
            <p class="title">Esse arquivo não parece uma exportação do WhatsApp</p>
            <p class="body2">{notChat ? "Esperávamos um .zip ou uma pasta com o .txt da conversa dentro." : app.error}</p>
          </div>
          <div class="actions">
            <button class="btn-1" onclick={pickZip}>Escolher outro arquivo</button>
          </div>
        </div>
      {:else}
        <div class="drop">
          <span class="tile"><Icon icon={Upload} size={24} stroke={1.8} /></span>
          <p class="title">Arraste o .zip da conversa para cá</p>
          <div class="actions">
            <button class="btn-1" title="Ctrl+O" onclick={pickZip}>Escolher .zip</button>
            <button class="btn-2" onclick={pickFolder}>Escolher pasta</button>
          </div>
        </div>
      {/if}

      <p class="note">
        <Icon icon={Lock} size={14} stroke={2} />
        <span>Processado só neste computador, sem internet e sem login. Guardamos apenas as estatísticas, nunca o texto das mensagens.</span>
      </p>
    </section>

    {#if firstUse}
      <aside class="col first">
        <h2 class="caps">Como exportar do celular</h2>
        <div class="first-body">
          {@render phones()}
          <ol class="big-steps">
            {#each FIRST_GUIDE[phone] as [title, text], i (title)}
              <li><span class="n">{i + 1}</span><span class="step"><b>{title}</b><span>{text}</span></span></li>
            {/each}
          </ol>
        </div>
      </aside>
    {:else}
      <aside class="col">
        <section class="recents">
          <h2>Recentes</h2>
          {#if recents.length}
            <ul>
              {#each recents as p (p)}
                {@const name = contact(p)}
                {@const fav = favs.includes(p)}
                <li class="row">
                  <button class="open" title={p} onclick={() => openFolder(p)}>
                    <span class="avatar">{name.charAt(0).toUpperCase()}</span>
                    <span class="info">
                      <span class="nm">{name}</span>
                      <span class="meta">{meta(p)}</span>
                    </span>
                  </button>
                  <button
                    class="act"
                    class:on={fav}
                    aria-pressed={fav}
                    aria-label={fav ? "Remover dos favoritos" : "Favoritar"}
                    title={fav ? "Remover dos favoritos" : "Favoritar"}
                    onclick={() => toggleFavorite(p)}
                  >
                    <svg width="16" height="16" viewBox="0 0 24 24" fill={fav ? "currentColor" : "none"} stroke="currentColor" stroke-width="1.6" stroke-linejoin="round" aria-hidden="true">
                      <path d="M12 3.6l2.55 5.17 5.7.83-4.12 4.02.97 5.68L12 16.62l-5.1 2.68.97-5.68L3.75 9.6l5.7-.83z" />
                    </svg>
                  </button>
                  <button class="act del" aria-label="Remover {name} dos recentes" title="Remover dos recentes" onclick={() => removeRecent(p, name)}>
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <path d="M4 7h16" /><path d="M9 7V4.8c0-.44.36-.8.8-.8h4.4c.44 0 .8.36.8.8V7" />
                      <path d="M6.5 7l.8 11.6c.07 1.05.95 1.9 2 1.9h5.4c1.05 0 1.93-.85 2-1.9L17.5 7" /><path d="M10.25 11v5.5" /><path d="M13.75 11v5.5" />
                    </svg>
                  </button>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-box">Nenhuma conversa recente.</p>
          {/if}
        </section>

        <section class="guide">
          <button class="toggle" aria-expanded={guideOpen} onclick={toggleGuide}>
            <span>Como exportar uma conversa do celular</span>
            <span class="chev" class:open={guideOpen}><Icon icon={ChevronDown} size={16} stroke={2} /></span>
          </button>
          {#if guideOpen}
            <div class="guide-body">
              {@render phones()}
              <ol class="steps">
                {#each GUIDE[phone] as [lead, rest] (lead)}
                  <li><b>{lead}</b>{rest}</li>
                {/each}
              </ol>
            </div>
          {/if}
        </section>
      </aside>
    {/if}
  </div>
</main>

<style>
  .home { display: flex; padding: 24px 24px 64px; font-family: "Inter Variable", system-ui, sans-serif; }
  .home :global(:focus-visible) { outline: 2px solid var(--text); outline-offset: 2px; border-radius: 8px; }
  .grid {
    margin: auto; width: 100%; max-width: 1040px;
    display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 48px; align-items: start;
  }
  .col { display: flex; flex-direction: column; gap: 24px; min-width: 0; }
  p, h1, h2, ul, ol { margin: 0; }

  h1 { font-size: 32px; line-height: 1.2; font-weight: 700; letter-spacing: -0.02em; color: var(--text); }
  h2 { min-height: 24px; display: flex; align-items: center; font-size: 14px; font-weight: 600; color: var(--text); }
  .caps { min-height: 18px; font-size: 12px; letter-spacing: 0.06em; text-transform: uppercase; color: var(--text-3); }

  .drop {
    min-height: 280px; padding: 32px; border-radius: 12px; background: var(--surface); border: 1.5px dashed var(--border-2);
    display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; gap: 20px;
  }
  .drop.busy { border-style: solid; }
  .drop.bad { border-color: var(--bad); }
  .tile { width: 48px; height: 48px; border-radius: 12px; background: var(--surface-2); color: var(--text); display: grid; place-items: center; }
  .bad .tile { background: var(--danger-surface); color: var(--bad); }
  .txt { display: flex; flex-direction: column; gap: 4px; max-width: 100%; }
  .title { font-size: 16px; font-weight: 600; color: var(--text); overflow-wrap: anywhere; }
  .body2 { font-size: 14px; line-height: 1.5; color: var(--text-2); }
  .actions { display: flex; flex-wrap: wrap; justify-content: center; gap: 8px; }

  .btn-1, .btn-2 { height: 40px; padding: 0 16px; border-radius: 8px; font-size: 14px; transition: background 150ms, opacity 150ms; }
  .btn-1 { background: var(--text); color: var(--on-ink); font-weight: 600; }
  .btn-1:hover { opacity: 0.88; }
  .btn-2 { border: 1px solid var(--border); color: var(--text); font-weight: 500; }
  .btn-2:hover { background: var(--surface-2); }

  .bar { width: 100%; max-width: 280px; height: 4px; border-radius: 2px; background: var(--border); overflow: hidden; }
  .bar i { display: block; height: 100%; background: var(--text); transform-origin: left; transition: transform 200ms; }

  .note { display: flex; align-items: flex-start; gap: 8px; font-size: 12px; line-height: 1.5; color: var(--text-3); }
  .note :global(svg) { margin-top: 2px; }

  .recents { display: flex; flex-direction: column; gap: 12px; }
  .recents ul { list-style: none; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  .row {
    display: flex; align-items: center; gap: 4px; padding-right: 8px; border-radius: 12px;
    border: 1px solid var(--border); background: var(--surface); transition: border-color 150ms, background 150ms;
  }
  .row:hover { border-color: var(--border-2); background: var(--surface-hover); }
  .open { flex: 1; min-width: 0; display: flex; align-items: center; gap: 12px; padding: 12px 8px 12px 16px; text-align: left; }
  .avatar { flex: none; width: 40px; height: 40px; border-radius: 50%; background: var(--surface-2); display: grid; place-items: center; font-size: 14px; font-weight: 600; color: var(--text); }
  .info { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .nm { font-size: 14px; font-weight: 600; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { font-size: 12px; line-height: 1.5; color: var(--text-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .act {
    flex: none; width: 32px; height: 32px; border-radius: 8px; display: grid; place-items: center; color: var(--text-2);
    opacity: 0; transition: opacity 150ms, background 150ms, color 150ms;
  }
  .row:hover .act, .act.on, .act:focus-visible { opacity: 1; }
  .act:hover { background: var(--surface-2); color: var(--text); }
  .act.on, .act.on:hover { color: var(--favorite); }
  .act.del:hover { color: var(--bad); }
  .empty-box { padding: 16px; border: 1px dashed var(--border); border-radius: 12px; font-size: 14px; line-height: 1.5; color: var(--text-3); }

  .guide { display: flex; flex-direction: column; padding-top: 12px; border-top: 1px solid var(--border); }
  .toggle { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 40px; width: 100%; text-align: left; font-size: 14px; font-weight: 600; color: var(--text); }
  .chev { color: var(--text-2); transition: transform 150ms; }
  .chev.open { transform: rotate(180deg); }
  .guide-body { display: flex; flex-direction: column; gap: 16px; padding-top: 12px; }

  .seg { display: inline-flex; align-self: flex-start; padding: 2px; border-radius: 8px; background: var(--surface); border: 1px solid var(--border); }
  .seg button { height: 28px; min-width: 72px; padding: 0 12px; border-radius: 6px; font-size: 12px; font-weight: 600; color: var(--text-2); transition: background 150ms, color 150ms; }
  .seg button.on { background: var(--surface-2); color: var(--text); }

  .steps { padding-left: 20px; display: flex; flex-direction: column; gap: 8px; font-size: 14px; line-height: 1.5; color: var(--text-2); }
  .steps b { font-weight: 600; color: var(--text); }

  .first { gap: 12px; }
  .first-body { display: flex; flex-direction: column; gap: 24px; padding-top: 4px; }
  .big-steps { list-style: none; padding: 0; display: flex; flex-direction: column; gap: 20px; }
  .big-steps li { display: flex; gap: 12px; }
  .n { flex: none; width: 24px; height: 24px; border-radius: 50%; background: var(--surface-2); display: grid; place-items: center; font-size: 12px; font-weight: 600; color: var(--text); }
  .step { display: flex; flex-direction: column; gap: 2px; font-size: 14px; line-height: 1.5; color: var(--text-2); }
  .step b { font-weight: 600; color: var(--text); }
</style>
