<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/logo-dark.svg">
    <img src=".github/logo.svg" alt="Spelunky" width="360">
  </picture>
</p>

<p align="center">
  Spelunky Classic, rebuilt in Rust.<br>
  <sub>An unofficial, modified port of Classic 1.1 for Linux and the web.</sub>
</p>

<p align="center">
  <a href="https://spelunky.twango.dev"><strong>Play in your browser</strong></a>
  &nbsp;·&nbsp;
  <a href="#run-locally">Run locally</a>
  &nbsp;·&nbsp;
  <a href="references/README.md">Original source</a>
</p>

<p align="center">
  <sub>Original game © 2008, 2009 Derek Yu and Mossmouth, LLC.<br>
  Distributed under the <a href="LICENSE">Spelunky User License</a>. Covered works may not be sold.</sub>
</p>

Original sprites and audio, a built-in level editor, and one Rust gameplay core for native and WebAssembly builds.

## Run locally

**Linux** — requires Rust, a graphical session, and the ALSA, udev, XKB, Wayland/X11 development libraries.

```sh
cargo run -p spelunky
```

**Browser** — requires Rust, Bun, `wasm-pack`, and the `wasm32-unknown-unknown` target.

```sh
bun install --frozen-lockfile
bun run dev
```

Open <http://127.0.0.1:5173>. Use `bun run build` for a static release.

<sub>Arrow keys to move · Z to jump · X to attack · B for bombs · R for ropes · Esc to pause · F2 for the editor. Bindings are configurable.</sub>
