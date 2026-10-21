<script lang="ts">
  import { onMount } from 'svelte';
  import { startGame } from './start-game';
  import { installPauseMenuState } from './pause-menu-state';

  let canvas: HTMLCanvasElement;
  let status: HTMLParagraphElement;
  let fullscreenStatus: HTMLParagraphElement;

  onMount(() => {
    const pauseMenu = installPauseMenuState(
      window as Window & { __spelunkyPauseMenuOpen?: unknown },
      (open) => {
        if (!open && fullscreenStatus) {
          fullscreenStatus.hidden = true;
          fullscreenStatus.textContent = '';
        }
      },
    );
    const clearFullscreenStatus = () => {
      fullscreenStatus.hidden = true;
      fullscreenStatus.textContent = '';
    };
    document.addEventListener('fullscreenchange', clearFullscreenStatus);
    void startGame(canvas, status);
    return () => {
      pauseMenu();
      document.removeEventListener('fullscreenchange', clearFullscreenStatus);
    };
  });
</script>

<canvas bind:this={canvas} id="spelunky-canvas" tabindex="0" aria-label="Spelunky Classic game"></canvas>
<p bind:this={fullscreenStatus} id="fullscreen-status" role="status" hidden></p>
<p bind:this={status} id="status" role="status">Loading Spelunky Classic…</p>
<p id="attribution">Unofficial modified port · Derek Yu / Mossmouth · <a href="assets/LICENSE" data-sveltekit-reload>License</a></p>

<style>
  :global(:root) { color-scheme: dark; font-family: system-ui, sans-serif; }
  :global(html), :global(body) { width: 100%; height: 100%; margin: 0; background: #111; }
  :global(body) { display: grid; grid-template-columns: minmax(0, 1fr); grid-template-rows: minmax(0, 1fr); place-items: center; overflow: hidden; }
  canvas { display: block; position: fixed; inset: 0; width: 100vw; height: 100vh; width: 100dvw; height: 100dvh; min-width: 0; min-height: 0; outline: none; touch-action: none; }
  #fullscreen-status { position: fixed; top: max(.75rem, env(safe-area-inset-top)); right: max(.75rem, env(safe-area-inset-right)); z-index: 2; max-width: min(24rem, calc(100vw - 1rem)); margin: 0; padding: .5rem .65rem; background: #211813; border: 1px solid #b67d4a; color: #eee; font-size: .8rem; }
  #status { position: absolute; max-width: 36rem; padding: 1.25rem; text-align: center; }
  :global(#status.error) { background: #211813; border: 1px solid #b67d4a; }
  :global(#status.ended) { position: fixed; inset: 0; z-index: 10; display: grid; place-items: center; box-sizing: border-box; width: 100vw; height: 100vh; height: 100dvh; max-width: none; margin: 0; padding: 2rem; background: #111; color: #eee; font-size: 1.1rem; }
  #attribution { position: absolute; right: .5rem; bottom: .3rem; margin: 0; font-size: .7rem; color: #ccc; }
  a { color: inherit; }
</style>
