export const AUDIO_UNLOCK_EVENT = 'spelunky-audio-gesture';

export function requestAudioUnlock(target: EventTarget = window): void {
  target.dispatchEvent(new Event(AUDIO_UNLOCK_EVENT));
}
