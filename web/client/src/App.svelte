<script lang="ts">
  import { onMount } from 'svelte';
  import { startGame } from './start-game';

  let canvas: HTMLCanvasElement;
  let status: HTMLParagraphElement;

  onMount(() => {
    void startGame(canvas, status);
  });
</script>

<canvas bind:this={canvas} id="spelunky-canvas" tabindex="0" aria-label="Spelunky Classic game"></canvas>
<p bind:this={status} id="status" role="status">Loading Spelunky Classic…</p>
<p id="attribution">Unofficial modified port · Derek Yu / Mossmouth · <a href="assets/COPYING.txt">License</a></p>

<style>
  :global(:root) { color-scheme: dark; font-family: system-ui, sans-serif; }
  :global(html), :global(body) { width: 100%; height: 100%; margin: 0; background: #111; }
  :global(body) { display: grid; grid-template-columns: minmax(0, 1fr); grid-template-rows: minmax(0, 1fr); place-items: center; overflow: hidden; }
  canvas { display: block; width: 100vw; height: 100vh; min-width: 0; min-height: 0; outline: none; touch-action: none; }
  #status { position: absolute; max-width: 36rem; padding: 1.25rem; text-align: center; }
  :global(#status.error) { background: #211813; border: 1px solid #b67d4a; }
  #attribution { position: absolute; right: .5rem; bottom: .3rem; margin: 0; font-size: .7rem; color: #ccc; }
  a { color: inherit; }
</style>
