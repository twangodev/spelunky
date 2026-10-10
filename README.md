# Spelunky Classic in Rust

An **unofficial, modified port of Spelunky Classic 1.1**. Original game copyright (c) 2008, 2009 Derek Yu and Mossmouth, LLC. Distributed under the [Spelunky User License](COPYING.txt); covered works may not be sold.

The same Rust gameplay core runs natively on Linux and in browsers through WebAssembly and WebGL 2. The runtime uses the original sprites and audio, without GameMaker or the original executable.

## Run

Linux needs a graphical session and the native development libraries listed in [development.md](docs/development.md).

```sh
cargo run -p spelunky
```

For the browser, install the `wasm32-unknown-unknown` target and `wasm-pack`, then:

```sh
./tools/build_web.sh
python3 tools/serve_web.py
```

Open `http://127.0.0.1:8000/`. The generated `web/` bundle can be served by a static HTTP server. WebGL 2 is required; WebGPU is not required.

## Play and edit

Use arrow keys to move, **Z** to jump, **X** to whip/use/throw, **Shift** to run, **C** to cycle inventory, **B** for bombs, **R** for ropes, **F** for flares, and **P** to purchase. Hold down and attack to pick up an item; hold up and attack to open containers. Escape pauses; F4 toggles fullscreen. Keyboard bindings, controllers, volume, and presentation settings are configurable.

The campaign includes all four areas, special destinations and Olmec. The original hub, records room, Sun/Moon/Stars challenges, trophies, and unlockable characters are available from the title menu. Use F2 or the editor menu to paint levels, edit metadata/signs/door links, undo/redo, save, import/export Classic `.lvl` files, and playtest. Linux also accepts `.lvl` files as command-line arguments or dropped files.

After eligible area exits, the Tunnel Man donation screen lets you fund shortcuts; partial payments persist between runs. Use up/down to select an amount, attack/purchase to donate, and Enter or Escape to continue.

Classic rules are the default. The optional Assisted preset adds 100 ms jump buffering and coyote time and keeps separate run records.

## High-refresh presentation

Gameplay advances at the original **30 Hz**. Entity motion and the camera interpolate at the display's refresh rate, with crisp textures and fractional positioning at output resolution. Rendering does not change gameplay speed or sprite animation durations. An original pixel-snapped mode is available; native builds also provide frame-cap and vsync controls.

Actual 240 Hz pacing depends on hardware and has not been measured in this headless environment. See the [validation evidence](docs/browser-validation.md) and [source fidelity review](docs/fidelity-review.md) for the distinction between implemented behavior, local tests, browser checks, and remaining acceptance work. This port does not claim exact GameMaker replay or pixel parity.

## Development

The workspace separates `spelunky-core`, the native/browser application, and `spelunky-tools`. Original source, provenance, and checksums are archived in [references](references/README.md). The asset exporter verifies every packed sprite pixel against the original data.

```sh
python3 tools/export_assets.py --verify
cargo test --workspace --locked
cargo fmt --all -- --check
cargo run -p spelunky-tools -- replay 42 600 classic
```

See [development](docs/development.md), [replay tools](docs/replay.md), [editor compatibility](docs/editor.md), and the subsystem status documents for implementation details and known differences. No deployment or publication is performed by these commands.
