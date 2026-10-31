<script lang="ts">
  let { name, color }: { name: string; color: string } = $props();
</script>

<span class="who" style:--c={color} title={name}><i class="dot"></i><span class="nm">{name}</span></span>
