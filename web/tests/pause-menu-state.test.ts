// @ts-expect-error Bun provides this built-in module to its test runner.
import { test } from 'bun:test';
import { installPauseMenuState, PAUSE_MENU_STATE_EVENT } from '../src/lib/pause-menu-state';

class PauseMenuTarget extends EventTarget {
  __spelunkyPauseMenuOpen: unknown = false;
}

function pauseMenuStateFollowsHostAndUnsubscribes(): void {
  const target = new PauseMenuTarget();
  const states: boolean[] = [];
  const dispose = installPauseMenuState(target, (open) => states.push(open));

  target.__spelunkyPauseMenuOpen = true;
  target.dispatchEvent(new Event(PAUSE_MENU_STATE_EVENT));
  target.__spelunkyPauseMenuOpen = false;
  target.dispatchEvent(new Event(PAUSE_MENU_STATE_EVENT));
  dispose();
  target.__spelunkyPauseMenuOpen = true;
  target.dispatchEvent(new Event(PAUSE_MENU_STATE_EVENT));

  const expected = [false, true, false];
  if (JSON.stringify(states) !== JSON.stringify(expected)) {
    throw new Error(`Pause menu state events expected ${JSON.stringify(expected)}, got ${JSON.stringify(states)}`);
  }
}

test('browser pause controls follow the host screen and detach on cleanup', pauseMenuStateFollowsHostAndUnsubscribes);
