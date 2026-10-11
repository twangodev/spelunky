<script lang="ts">
  import { onMount } from 'svelte';
  import { startGame } from './start-game';
  import { installFullscreenControls } from './fullscreen';

  let canvas: HTMLCanvasElement;
  let status: HTMLParagraphElement;
  let fullscreenStatus: HTMLParagraphElement;
  let fullscreenAvailable = $state(false);
  let fullscreenActive = $state(false);
  let fullscreenPending = $state(false);
  let toggleFullscreen = () => {};

  onMount(() => {
    const fullscreen = installFullscreenControls(canvas, fullscreenStatus, (state) => {
      fullscreenAvailable = state.available;
      fullscreenActive = state.active;
      fullscreenPending = state.pending;
    });
    toggleFullscreen = fullscreen.toggle;
    void startGame(canvas, status);
    return fullscreen.dispose;
  });
</script>

<canvas bind:this={canvas} id="spelunky-canvas" tabindex="0" aria-label="Spelunky Classic game"></canvas>
<button
  id="fullscreen-toggle"
  type="button"
  disabled={!fullscreenAvailable || fullscreenPending}
  aria-controls="spelunky-canvas"
  aria-describedby="fullscreen-status"
  title={fullscreenAvailable ? (fullscreenActive ? 'Exit fullscreen (Esc)' : 'Enter fullscreen') : 'Fullscreen is unavailable in this browser'}
  onpointerdown={(event) => event.preventDefault()}
  onclick={() => toggleFullscreen()}
>{fullscreenActive ? 'Exit fullscreen' : 'Fullscreen'}</button>
<p bind:this={fullscreenStatus} id="fullscreen-status" role="status" hidden></p>
<p bind:this={status} id="status" role="status">Loading Spelunky Classic…</p>
<p id="attribution">Unofficial modified port · Derek Yu / Mossmouth · <a href="assets/COPYING.txt" data-sveltekit-reload>License</a></p>

<style>
  :global(:root) { color-scheme: dark; font-family: system-ui, sans-serif; }
  :global(html), :global(body) { width: 100%; height: 100%; margin: 0; background: #111; }
  :global(body) { display: grid; grid-template-columns: minmax(0, 1fr); grid-template-rows: minmax(0, 1fr); place-items: center; overflow: hidden; }
  canvas { display: block; position: fixed; inset: 0; width: 100vw; height: 100vh; width: 100dvw; height: 100dvh; min-width: 0; min-height: 0; outline: none; touch-action: none; }
  #fullscreen-toggle { position: fixed; bottom: max(.5rem, env(safe-area-inset-bottom)); left: max(.5rem, env(safe-area-inset-left)); z-index: 1; padding: .4rem .65rem; border: 1px solid #777; border-radius: .25rem; background: #111d; color: #eee; font: inherit; font-size: .8rem; cursor: pointer; }
  #fullscreen-toggle:focus-visible { outline: 2px solid #e8b34e; outline-offset: 2px; }
  #fullscreen-toggle:disabled { opacity: .55; cursor: default; }
  #fullscreen-status { position: fixed; bottom: calc(max(.5rem, env(safe-area-inset-bottom)) + 2.5rem); left: max(.5rem, env(safe-area-inset-left)); z-index: 1; max-width: min(24rem, calc(100vw - 1rem)); margin: 0; padding: .5rem .65rem; background: #211813; border: 1px solid #b67d4a; color: #eee; font-size: .8rem; }
  #status { position: absolute; max-width: 36rem; padding: 1.25rem; text-align: center; }
  :global(#status.error) { background: #211813; border: 1px solid #b67d4a; }
  #attribution { position: absolute; right: .5rem; bottom: .3rem; margin: 0; font-size: .7rem; color: #ccc; }
  a { color: inherit; }
</style>
