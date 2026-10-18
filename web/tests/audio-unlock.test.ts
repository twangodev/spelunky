// @ts-expect-error Bun provides this built-in module to its test runner.
import { test } from 'bun:test';
import { AUDIO_UNLOCK_EVENT, requestAudioUnlock } from '../src/lib/audio-unlock';

function enableSoundDispatchesTheHostGestureEvent(): void {
  const target = new EventTarget();
  let received = false;
  target.addEventListener(AUDIO_UNLOCK_EVENT, () => { received = true; });

  requestAudioUnlock(target);

  if (!received) {
    throw new Error('The enable-sound gesture must notify the Rust audio host');
  }
}

test('enable-sound gesture dispatches the event consumed by the Rust host', enableSoundDispatchesTheHostGestureEvent);
