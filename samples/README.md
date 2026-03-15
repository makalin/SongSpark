# Sample files & sound banks

Keep your WAV/MP3 files in this **samples** folder (e.g. `Drums/`, `808/`, `FX/`). **Do not rename or delete them** — they are used by **packs** (see below). You can add more samples and folders anytime.

- **Build**: Trunk copies this folder into the build output (see `index.html` copy-dir) so the app can load samples. Your originals in `samples/` are never changed.
- **Example pack**: In the app, click **Load example pack** to load the built-in pack that maps names like `bd`, `sd`, `hh`, `cp` to your actual files (e.g. `Drums/Kick 1.0.wav`, `Drums/Snare I.wav`). You can add more packs in the `packs/` folder.

SongSpark follows **Strudel/TidalCycles**-style naming. See [Strudel Samples](https://urswilke.github.io/strudel/learn/samples/) for the full reference.

## Sound banks (Strudel-style)

Use **multiple banks** (e.g. "RolandTR808", "Live drum", "Techno") and switch between them for playback. You can:

- **Upload files** — Add a pack, then choose WAV/MP3 files. Name them with the short names below.
- **Sample pack format: `.jsamp`** (sample pack = bank definition). Use this extension so pack files are not confused with song files (`.jsong`).
- **Load pack from URL** — Enter a URL to a **.jsamp** (or .json) file. Structure:
  ```json
  {
    "bankName": "TR808",
    "baseUrl": "https://example.com/samples/",
    "metadata": { "composer": "...", "albumCoverUrl": "...", "description": "...", "link": "..." },
    "samples": { "bd": "bd/kick.wav", "sd": "sd/snare.wav", "hh": "hh/closed.wav", "cp": "cp/clap.wav" }
  }
  ```
  `baseUrl` must end with `/`. **metadata** is optional. The app shows composer, cover, description and link when a pack is selected.
- **Load pack from file** — In the app, click **Load pack from file (.jsamp)** and choose a `.jsamp` file. Relative `baseUrl` (e.g. `./samples/`) is resolved against the app origin.
- **Samples folder metadata** — Optional `metadata.json` in this folder (same shape as pack **metadata**).

## Drum names (Strudel/Tidal convention)

Same as [Strudel's default sample map](https://urswilke.github.io/strudel/learn/samples/):

| Abbreviation | Sound |
|--------------|--------|
| `bd` | Bass drum / Kick |
| `sd` | Snare drum |
| `rim` | Rimshot |
| `cp` | Clap |
| `hh` | Closed hi-hat |
| `oh` | Open hi-hat |
| `cr` | Crash |
| `rd` | Ride |
| `sh` | Shakers / maracas / cabasa |
| `ht` | High tom |
| `mt` | Medium tom |
| `lt` | Low tom |
| `cb` | Cowbell |
| `tb` | Tambourine |
| `perc` | Other percussion |
| `misc` | Miscellaneous |
| `fx` | Effects |

Name your files accordingly (e.g. `bd.wav`, `sd.wav`, `hh.wav`) so patterns and presets work.

## Where to get samples

- [Dirt-Samples](https://github.com/tidalcycles/Dirt-Samples) (GitHub) — used by Strudel/Tidal; you can host a copy and point `baseUrl` at it.
- Your own WAV/MP3 in the **samples** folder or any server.
- Free packs: search "free drum samples wav", "808 909 one-shots".

## How to play in the app

1. **Banks** — Add a pack (or load from URL with the JSON above). Switch the **Pack** dropdown to change sound.
2. **Pattern** — Presets dropdown or type e.g. `bd sd hh*2 cp` (`*n` = repeat).
3. **Play** — Play button or **Spacebar**. Use **MIDI** (if enabled) to drive external devices.
4. **Visuals** — The step view and visualizer are driven by the pattern and BPM (code-driven animation).

**Channels (tracks)** — Use multiple lines so each line is its own track with its own step density. Start a line with `@N`: steps per bar (e.g. `@4` = quarter notes, `@16` = 16ths). Example: first line `@4 bd sd bd sd` (simple drums, 4 hits per bar), second line `@16 A4 C5 E5 G5 ...` (fast melody, 16 notes per bar). All tracks play in parallel. No need to assign instruments to channels: samples play from the active pack, note names (A4, C#5) play from the synth.

No bundled samples; add your own or load from a URL.

## How to load and play a drum loop from code

Any sample in the **active pack** can be triggered by typing its **name** in the code view. To use a full drum loop (e.g. a 1- or 4-bar WAV):

1. **Put the loop file in `samples/`**  
   Example: `samples/Loops/break.wav` or `samples/drum-loop.wav`.

2. **Add it to a pack** under a short name:
   - **Option A – Pack from file**: Create or edit a `.jsamp` in `packs/` and add an entry in `samples`:
     ```json
     "samples": {
       "bd": "Drums/Kick 1.0.wav",
       "sd": "Drums/Snare I.wav",
       "loop": "Loops/break.wav"
     }
     ```
     Then in the app: **Load pack from file (.jsamp)** and choose that pack.
   - **Option B – Load example pack**: If your loop is under `samples/`, add `"loop": "Loops/break.wav"` to the built-in example pack JSON (in the app source, e.g. `packs/example.jsamp` or the embedded pack), then load the example pack.
   - **Option C – New pack + Upload**: Create a new pack name, then **Upload files** and assign the loop file to a name like `loop` or `drum-loop`.

3. **Select that pack** in the **Pack** dropdown so it’s the active pack.

4. **In the code view**, type the name as a token. It will play at that step time:
   - **Once per pattern cycle**: put the loop at the start, e.g. `loop bd sd hh cp` — the loop plays at time 0 and runs for its full length (1 bar, 4 bars, etc.) while the one-shots play on the grid.
   - **Together with other sounds**: e.g. `loop|A4` to play the loop and a synth note at the same time.
   - **Only the loop**: e.g. `loop` alone — it plays at the start of every loop.

The loop sample plays from start to finish when that token is triggered; BPM only affects when the *pattern* repeats, not the playback speed of the loop file.
