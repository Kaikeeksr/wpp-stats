<script lang="ts">
  import { api } from "../lib/api";

  let { id }: { id: number } = $props();
  let url = $state("");

  $effect(() => {
    let live = true;
    let own = "";
    api
      .sticker(id)
      .then((b) => {
        if (!live) return;
        own = URL.createObjectURL(new Blob([b], { type: "image/webp" }));
        url = own;
      })
      .catch(() => {});
    return () => {
      live = false;
      if (own) URL.revokeObjectURL(own);
    };
  });
</script>

{#if url}<img src={url} alt="Figurinha" />{:else}<span></span>{/if}

<style>
  img, span { display: block; width: 56px; height: 56px; object-fit: contain; }
  span { border-radius: var(--r-sm); background: var(--active); }
</style>
