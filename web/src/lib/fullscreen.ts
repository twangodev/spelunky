type FullscreenState = {
  available: boolean;
  active: boolean;
  pending: boolean;
};

export function installFullscreenControls(
  canvas: HTMLCanvasElement,
  feedback: HTMLParagraphElement,
  onStateChange: (state: FullscreenState) => void,
): { toggle: () => void; dispose: () => void } {
  const available = Boolean(document.fullscreenEnabled
    && typeof document.documentElement.requestFullscreen === 'function'
    && typeof document.exitFullscreen === 'function');
  let pending = false;
  let disposed = false;

  function publishState(): void {
    if (!disposed) {
      onStateChange({ available, active: Boolean(document.fullscreenElement), pending });
    }
  }

  function focusCanvas(): void {
    if (!disposed && document.hasFocus() && canvas.isConnected) {
      canvas.focus({ preventScroll: true });
    }
  }

  function showError(): void {
    if (!disposed) {
      feedback.textContent = 'Could not change fullscreen. Use your browser’s fullscreen controls.';
      feedback.hidden = false;
    }
  }

  function fullscreenChanged(): void {
    feedback.hidden = true;
    feedback.textContent = '';
    publishState();
    focusCanvas();
  }

  async function changeFullscreen(): Promise<void> {
    pending = true;
    feedback.hidden = true;
    feedback.textContent = '';
    publishState();
    try {
      // Both this button and the Rust fullscreen controls target the same root.
      if (document.fullscreenElement) {
        await document.exitFullscreen();
      } else {
        await document.documentElement.requestFullscreen();
      }
    } catch {
      showError();
    } finally {
      pending = false;
      publishState();
      focusCanvas();
    }
  }

  document.addEventListener('fullscreenchange', fullscreenChanged);
  document.addEventListener('fullscreenerror', showError);
  publishState();

  return {
    toggle() {
      if (available && !pending && !disposed) {
        void changeFullscreen();
      }
    },
    dispose() {
      disposed = true;
      document.removeEventListener('fullscreenchange', fullscreenChanged);
      document.removeEventListener('fullscreenerror', showError);
    },
  };
}
