export const PAUSE_MENU_STATE_EVENT = 'spelunky-pause-menu-state';

type PauseMenuStateTarget = EventTarget & {
  __spelunkyPauseMenuOpen?: unknown;
};

/** Mirror the Rust host's pause state so fullscreen feedback clears on resume. */
export function installPauseMenuState(
  target: PauseMenuStateTarget,
  onChange: (open: boolean) => void,
): () => void {
  function publish(): void {
    onChange(target.__spelunkyPauseMenuOpen === true);
  }

  target.addEventListener(PAUSE_MENU_STATE_EVENT, publish);
  publish();

  return () => target.removeEventListener(PAUSE_MENU_STATE_EVENT, publish);
}
