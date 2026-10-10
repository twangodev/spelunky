type WasmGame = {
  default: () => Promise<unknown>;
  start: () => void | Promise<void>;
};

export async function startGame(canvas: HTMLCanvasElement, status: HTMLParagraphElement): Promise<void> {
  try {
    if (!document.createElement('canvas').getContext('webgl2')) {
      throw new Error('WebGL 2 is required. Enable hardware acceleration or use a browser with WebGL 2 support.');
    }
    // Keep wasm-pack's JavaScript and Wasm together, unmodified, at their original URLs.
    const moduleUrl = new URL('./pkg/spelunky.js', document.baseURI).href;
    const { default: init, start } = await import(/* @vite-ignore */ moduleUrl) as WasmGame;
    await init();
    await start();
    status.hidden = true;
    canvas.focus();
  } catch (error) {
    status.classList.add('error');
    status.textContent = `Spelunky could not start: ${error instanceof Error ? error.message : String(error)}`;
    console.error(error);
  }
}
