# Spelunky Classic 1.1 reference material

Copyright (c) 2008, 2009 Derek Yu and Mossmouth, LLC. This repository is an unofficial modified port. Original source, sprites, music, sound effects and related material remain covered by the [Spelunky User License](original/source/COPYING.txt); this license must accompany redistribution. Covered works may not be sold.

Downloaded from the links on the [official Classic page](https://spelunkyworld.com/original.html) on 2026-10-10:

- `original/spelunky_1_1_src.zip`: https://www.derekyu.com/games/spelunky_1_1_src.zip
- `original/spelunky_1_1.zip`: https://www.derekyu.com/games/spelunky_1_1.zip
- `original/source/`: exact source archive contents, including original license and readme.
- `original/game/`: non-executable audio, text, and configuration from the game archive. The Windows executable and DLL remain only inside the original archive and are never executed by the tools.
- `classic-1.1/`: official `spelunky.gmk` exported with GmkSplitter v0.21, preserving all resource and instance IDs. This is the reference for translation; the community update project was used for discovery only.
- `source-index.json`: generated resource metadata and event inventory. This inventories source coverage; it does not claim that the Rust implementation has ported these features.
- `SHA256SUMS.json`: SHA256 hashes of all reference files (excluding the manifest itself).

Export command:

```sh
java -jar gmksplit.jar --preserve-ids all --charset windows-1252 \
  references/original/source/spelunky.gmk references/classic-1.1
```

[GmkSplitter v0.21](https://github.com/Medo42/Gmk-Splitter/releases/tag/V0.21) was used only as an external build-time extraction tool. Its JAR SHA256 is `319b9b0bf7a724059a18dfbe167b4057ec9d32d8269bf5ff4e0b2a6aed5ad3ed`; release ZIP SHA256 is `4418aede1315057828b5102e7e1345ef60fb65b370beb6650db9fcf70e675fe9`. The tool and temporary Java runtime are not vendored or runtime dependencies.

Run `python3 tools/acquire_reference.py --offline` to verify the archive checksums and regenerate the reference hash manifest. To reproduce a missing export, supply `--splitter /path/to/gmksplit.jar --java /path/to/java`. Existing exports are preserved, so use a fresh checkout for clean reproduction. Run `python3 tools/index_reference.py` after export to regenerate the resource index.

Original archives:

| File | SHA256 |
| --- | --- |
| Source ZIP | `ad7d23e8de6a1398d245d228869d5c5bbd2c0e72910d66539c21306265cf7014` |
| Game ZIP | `572f0a1976e76ebbd295daae651b8b6f00c6b9cf960d3e1c2cc57128d6437727` |
| spelunky.gmk | `d28e322e4aa03b0137485d2c1c3be11899502f2b582b3d22b8939be81f97186c` |
