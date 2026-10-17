export async function startGame(canvas: HTMLCanvasElement, status: HTMLParagraphElement): Promise<void> {
  try {
    if (!document.createElement('canvas').getContext('webgl2')) {
      throw new Error('WebGL 2 is required. Enable hardware acceleration or use a browser with WebGL 2 support.');
    }
    const { default: init, start } = await import('virtual:spelunky');
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
