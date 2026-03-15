# Song examples

**File format: `.jsong`** (song = pattern + metadata). Keeps song examples clearly separate from sample packs (`.jsamp`).

Each `.jsong` file has:

- **code** – Pattern code (e.g. `bd sd hh*2 cp`) — this is what appears in the editor
- **title**, **bpm**, **composer**, **description**, **link** – Optional metadata (shown in “Song info” when loaded)

**Load:** Use the **Examples** dropdown (loads code + BPM + metadata), or **Load** and choose a `.jsong` or session `.json` file. The editor shows only the **code**; other details appear in the **Song info** box.

## Files

- `presets.json` – **Pattern presets** (Presets dropdown). Edit to add or change presets.
- `index.json` – List of example ids and `.jsong` file names (Examples dropdown).
- `*.jsong` – Song examples: Complex House, Techno Loop, Breakbeat Advanced, Hip-Hop Groove, Drum & Bass Loop, Reggae One Drop, Disco Groove, Trap Beat. Add more and register in `index.json`.

**Build:** Trunk copies this folder into the build output (see `index.html` copy-dir).
