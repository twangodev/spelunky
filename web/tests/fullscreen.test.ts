// @ts-expect-error Bun provides this built-in module to its test runner.
import { test } from 'bun:test';
import { installFullscreenControls } from '../src/lib/fullscreen';

function equal(actual: unknown, expected: unknown, message: string): void {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

function unsupportedFullscreenStaysInactive(): void {
  const originalDocument = Object.getOwnPropertyDescriptor(globalThis, 'document');
  const unsupportedDocument = {
    documentElement: {},
    hasFocus: () => true,
    addEventListener: () => {},
    removeEventListener: () => {},
  } as unknown as Document;
  Object.defineProperty(globalThis, 'document', {
    configurable: true,
    value: unsupportedDocument,
  });

  try {
    let state: { available: boolean; active: boolean; pending: boolean } | undefined;
    const canvas = { isConnected: true, focus: () => {} } as unknown as HTMLCanvasElement;
    const feedback = { hidden: true, textContent: '' } as unknown as HTMLParagraphElement;
    const controls = installFullscreenControls(canvas, feedback, (next) => { state = next; });

    equal(state, { available: false, active: false, pending: false },
      'A browser without the Fullscreen API must report an inactive, unavailable state');
    controls.toggle();
    equal(state, { available: false, active: false, pending: false },
      'Toggling unavailable fullscreen must leave the state inactive');
    controls.dispose();
  } finally {
    if (originalDocument) Object.defineProperty(globalThis, 'document', originalDocument);
    else Reflect.deleteProperty(globalThis, 'document');
  }
}

test('fullscreen stays inactive when the browser has no Fullscreen API', unsupportedFullscreenStaysInactive);
