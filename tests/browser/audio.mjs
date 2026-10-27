// Observe the actual game graph alongside its normal output; no audio device capture.
import assert from 'node:assert/strict';

export function installAudioProbe() {
  const upstream = new WeakMap();
  const probes = [];
  const connect = AudioNode.prototype.connect;
  let serial = 0;
  function detach(probe) {
    try { probe.node.disconnect(probe.analyser); } catch { /* game already disconnected */ }
    probe.analyser.disconnect();
  }
  AudioNode.prototype.connect = function (destination, ...args) {
    const result = connect.call(this, destination, ...args);
    if (destination instanceof AudioNode) {
      upstream.set(destination, this);
      if (destination === this.context.destination && this instanceof GainNode) {
        const panner = upstream.get(this);
        const source = panner && upstream.get(panner);
        const kind = source instanceof AudioBufferSourceNode ? 'effect'
          : source instanceof MediaElementAudioSourceNode ? 'music' : null;
        if (kind) {
          const analyser = this.context.createAnalyser();
          analyser.fftSize = 2048;
          connect.call(this, analyser);
          if (probes.length >= 64) detach(probes.shift());
          probes.push({ kind, analyser, node: this, context: this.context, peak: 0, serial: ++serial });
        }
      }
    }
    return result;
  };
  window.__audioProbe = probes;
  const timer = setInterval(() => {
    for (const probe of probes) {
      const samples = new Float32Array(probe.analyser.fftSize);
      probe.analyser.getFloatTimeDomainData(samples);
      for (const sample of samples) probe.peak = Math.max(probe.peak, Math.abs(sample));
    }
  }, 10);
  window.__audioProbeSerial = () => serial;
  window.__audioProbeCleanup = () => {
    clearInterval(timer);
    AudioNode.prototype.connect = connect;
    for (const probe of probes.splice(0)) detach(probe);
  };
}

export async function runAudioAcceptance({ page, result }) {
  // Startup reaches the title through real Playwright key presses; start a
  // campaign and jump through ordinary gameplay input.
  try {
    await page.keyboard.press('Enter');
    await page.waitForFunction(() => window.__spelunkyTest?.screen === 'Playing', null, { timeout: 15000 });
    await page.waitForFunction(() => window.__spelunkyTest?.player?.grounded && !window.__spelunkyTest?.opening, null, { timeout: 30000 });
    const { before, baseline } = await page.evaluate(() => ({
      before: window.__spelunkyTest.player.y,
      baseline: window.__audioProbeSerial(),
    }));
    await page.keyboard.down('z');
    try {
      await page.waitForFunction(y => window.__spelunkyTest?.player?.y < y - 1, before, { timeout: 5000 });
    } finally {
      await page.keyboard.up('z').catch(() => {});
    }
    await page.waitForFunction(baseline =>
      window.__audioProbe.some(probe => probe.kind === 'music' && probe.peak > 0.000001)
      && window.__audioProbe.some(probe => probe.kind === 'effect' && probe.serial > baseline && probe.peak > 0.000001), baseline, { timeout: 10000 });
    const observations = await page.evaluate(baseline => window.__audioProbe.map(({ kind, context, peak, serial }) => ({ kind, state: context.state, peak, afterBaseline: serial > baseline })), baseline);
    assert.ok(observations.some(probe => probe.kind === 'effect' && probe.afterBaseline && probe.state === 'running' && probe.peak > 0.000001));
    assert.ok(observations.some(probe => probe.kind === 'music' && probe.state === 'running' && probe.peak > 0.000001));
    result.audio = observations;
    result.checks.push('Post-gain samples from streamed music and a new decoded effect after a real jump are nonzero');
  } catch (error) {
    result.audio = await page.evaluate(() => window.__audioProbe.map(({ kind, context, peak }) => ({ kind, state: context.state, peak }))).catch(() => []);
    throw error;
  } finally {
    await page.evaluate(() => window.__audioProbeCleanup()).catch(() => {});
  }
}
