# WWM Overlay – MIDI Music Player

A beautiful, feature-rich MIDI music player for **Where Winds Meet** that plays your songs by automatically pressing the correct keyboard keys in-game.

> ⚠️ **WARNING: Use at Your Own Risk**
> The publisher has started banning players for using third-party tools (a broad category that includes many types of programs). It is unclear whether MIDI players specifically trigger bans, but the risk exists. Chinese server players have not yet reported any bans for using MIDI players. Proceed with caution.

<p align="center">
  <img width="780" alt="WWM Overlay screenshot" src="https://github.com/user-attachments/assets/8977b742-7f7d-47d9-b78f-d36ed677e3c5" />
</p>

<p align="center">
  <!-- Update these badge links to match your repo if needed -->
  <a href="#"><img src="https://img.shields.io/badge/platform-Windows-blue" /></a>
  <a href="#"><img src="https://img.shields.io/badge/built%20with-Rust%20%26%20Svelte-orange" /></a>
  <a href="#"><img src="https://img.shields.io/badge/framework-Tauri-7952B3" /></a>
</p>

> **Note:**  
> **36-key mode** uses instant **Shift/Ctrl combos** for sharps and flats.  
> If notes are dropping, try increasing the **modifier delay** in **Settings → Input**.

---

## ✨ Demo

https://github.com/user-attachments/assets/4d25e203-0e4f-4b0f-8dc4-e855ce5e6647  

https://github.com/user-attachments/assets/5223ff30-a859-4433-84c0-bfb3d8a8ed46  

### 🟢 Mini Mode

Collapse the app into a **tiny floating icon** while playing.  
The icon glows **green** when music is playing.  
Press **`Insert`** to toggle mini mode or use the minimize button in the sidebar.

<p align="center">
  <img width="64" height="89" alt="Mini mode icon" src="https://github.com/user-attachments/assets/f0de318f-6a1a-4e92-93c8-ba73b42d4d13" />
</p>

---

## 📚 Table of Contents

- [What is this?](#-what-is-this)
- [Support](#-support)
- [Features](#-features)
- [Getting Started](#-getting-started)
  - [First Time Setup](#first-time-setup)
  - [Playing Music](#playing-music)
- [Keyboard Shortcuts](#-keyboard-shortcuts-global-hotkeys)
- [Note Modes & Key Modes](#-note-calculation-modes)
- [Cloud Gaming Mode](#-cloud-gaming-mode)
- [Song Library (P2P Sharing)](#-song-library-p2p-sharing)
- [Library Management](#-library-management)
- [Settings & Customization](#-settings--customization)
- [Band Mode (Experimental)](#-band-mode-experimental)
- [In-App Controls](#-in-app-controls)
- [Managing Playlists](#-managing-playlists)
- [Tips](#-tips)
- [Troubleshooting](#-troubleshooting)
- [MIDI Folder Structure](#-where-to-put-midi-files)
- [Building from Source](#-building-from-source)
- [Credits](#-credits)

---

## 🎮 What is this?

**WWM Overlay – MIDI Music Player** lets you play music in **Where Winds Meet**’s music minigame using your own **MIDI files**.

- Load `.mid` files into the app
- Hit **Play**
- The app automatically sends the correct **keyboard keys** to the game

It’s basically an **auto-play piano** for the in-game instrument, with a **modern Spotify-style UI** and lots of controls.

---

## ☕ Support

If you enjoy this app and want to support development:

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/snowiy)

---

## 🌟 Features

### 🎧 Player & UI

- **Spotify-style interface** – Dark theme, smooth animations
- **Easy to use** – Drag & drop your MIDI files, then click play
- **Mini mode** – Tiny floating icon that glows while playing
- **Always on top toggle** – Pin button to keep window above others
- **Real-time progress** – See where you are in the song
- **Seek support** – Click the timeline to jump to any position
- **Song info** – Shows BPM and difficulty (Easy / Medium / Hard / Expert)
- **Remembers window position** across sessions

### 🎼 Playback & Mapping

- **Multiple note modes** – 9 different note-mapping algorithms
- **21/36 key toggle** – Natural notes only or 36-key sharps/flats mode
- **Real-time mode switching** – Change note mode mid-song
- **Track selector** – Solo specific MIDI tracks (e.g., melody only)
- **Speed control** – 0.25× to 2× playback speed
- **Octave shift** – Shift pitch up/down by up to 2 octaves
- **Queue system** – Build a playlist and play in order
- **Loop / repeat** – Keep your favorite song running

### 🎵 Library & Playlists

- **Large library support** – Optimized for 10,000+ MIDI files
- **Favorites** – Mark songs and drag to reorder favorites
- **Multiple playlists** – Create, rename, and manage custom lists
- **Search & sort** – Search inside library, favorites, queue, playlists
- **Multi-select** – Ctrl+click / Shift+click for bulk operations
- **Import folders & ZIPs** – Bulk import MIDI files
- **Drag & drop reordering** – Queue, favorites, playlists
- **Custom album folder** – Set your own library directory

### ⌨️ Input & Hotkeys

- **Global hotkeys** – Control playback even while the game is focused
- **Play without focus (local mode)** – Game window can be unfocused
- **Custom keyboard layouts** – Presets for QWERTY, QWERTZ, AZERTY, plus fully custom bindings
- **Custom window detection** – Add your own process/window titles for game detection

### 🌐 Online Features

- **Song Library (P2P)** – Share and download songs from other users
- **Band Mode** (experimental) – Synchronized multi-player performance
- **Relay / TURN support** – For stricter NAT / network setups
- **Custom discovery server URL** – For advanced users

### 🌍 Internationalization

- **Multi-language support**: English, 日本語, 한국어, ไทย, 中文

---

## 🚀 Getting Started

### First Time Setup

1. **Download the app**  
   Get the latest release from the **Releases** page.

2. **Extract the files**  
   Unzip to any folder (e.g. `C:\Games\wwm-overlay`).

3. **Add your MIDI files**  
   Put your `.mid` files in the `album` folder (see [folder structure](#-where-to-put-midi-files)).

4. **Run as Administrator**  
   Right-click `wwm-overlay.exe` → **Run as administrator**.  
   > Needed to send keyboard input to the game.

---

### Playing Music

1. **Open the game**  
   Start **Where Winds Meet** and open the **music minigame**.

2. **Select a song**  
   In the app, click a song in your library.

3. **Add to queue**  
   Use the playlist icon to add songs to **queue** or **playlists**.

4. **Play**
   Click **Play** or press **`ScrollLock`**.

5. **Enjoy**  
   The app sends keys directly to the game window.  
   You can use your PC as normal in the background (local mode).

> 💡 **Tip:** In **Local mode**, the game window can be in the background (but not minimized).

---

## ⌨️ Keyboard Shortcuts (Global Hotkeys)

These work even when the game has focus:

| Key            | Action                |
|--------------:|-----------------------|
| **ScrollLock** | Play / Pause          |
| **F10**        | Previous track        |
| **F11**        | Next track            |
| **F12**        | Stop                  |
| **End**        | Stop (alternative)    |
| **`[`**        | Previous note mode    |
| **`]`**        | Next note mode        |
| **Insert**     | Toggle mini mode      |

---

## 🎼 Note Calculation Modes

The app offers **9 algorithms** for mapping MIDI notes to in-game keys:

| Mode              | Description                                                    |
|-------------------|----------------------------------------------------------------|
| **YueLyn**        | Recommended mode – YueLyn’s favorite all-round play mode      |
| **Closest**       | Finds the nearest available note (works well for most songs)  |
| **Wide**          | Uses higher & lower rows more often (wider spread)            |
| **Sharps**        | 36-key mode: prefers Shift/Ctrl sharps and flats              |
| **Quantize**      | Snaps to strict scale notes                                   |
| **Transpose Only**| Direct mapping with only octave shifting                      |
| **Pentatonic**    | 5-note pentatonic scale (do–re–mi–so–la)                      |
| **Chromatic**     | 12-semitone → 7-key detailed mapping                          |
| **Raw**           | Raw 1:1 mapping (`MIDI note % 21`), no extra processing       |

Change modes in real time via **`[` / `]`** or the **mode selector** in the bottom bar.

---

### 🎹 Key Modes (21 vs 36 Keys)

| Mode        | Description                                              |
|-------------|----------------------------------------------------------|
| **21 Keys** | Natural notes only (default, simpler & safer)           |
| **36 Keys** | Adds sharps/flats with **Shift/Ctrl** modifier combos   |

Toggle via the **“21 / 36”** button in the bottom bar.

In **36-key mode**, sharps/flats send key combos like `Shift+X` or `Ctrl+X` **instantly**.  
If notes are missing, increase the **modifier delay** in Settings.

---

## ☁️ Cloud Gaming Mode

Designed for **GeForce Now**, **Xbox Cloud Gaming**, and similar services.

| Mode            | How it works                     | Background play |
|-----------------|----------------------------------|-----------------|
| **Local**       | `PostMessage` to game window     | ✅ Yes          |
| **Cloud Gaming**| `SendInput` (global keyboard)    | ❌ No           |

⚠️ **Cloud Gaming Mode warnings:**

- Uses **SendInput** → sends real global keystrokes
- You **must** keep the cloud gaming window **focused**
- Don’t type in chat or other apps while playing
- **Background playback is not possible** in this mode

---

## 🌐 Song Library (P2P Sharing)

Share and discover MIDI files with other players using a built-in **peer-to-peer** library.

### How to Use

1. Go to the **Online → Share** tab
2. Enable **“Sharing”**
3. Choose which songs to share (all or selected)
4. Browse songs from others and click to download
5. Downloaded songs appear in your library

### Features

- Shows how many songs are available from others
- Already owned songs are marked **“Owned”**
- Real-time download progress
- Auto-reconnect & re-share after restart (if enabled)
- Custom discovery server URL (for advanced users)
- **Share picker** – full-screen UI with:
  - Alphabet navigation  
  - Search  
  - Batch selection  
- **Floating notifications** – bottom bar for download progress/errors

### Security

- Only **valid MIDI files** are accepted
- Executables (`.exe`, scripts, ELF, Mach-O, etc.) are blocked
- MIDI header validation (`MThd` / `MTrk` required)
- Max file size: **50 MB**
- Filenames sanitized to prevent path traversal

### Privacy

- Only **file names & hashes** are shared for discovery
- File contents transfer **directly P2P**
- No account or login required

---

## 🗂 Library Management

Right-click any song in your library to:

- **Rename** – Change its file name
- **Delete** – Remove with confirmation
- **Open Location** – Open its folder in Explorer

---

## ⚙️ Settings & Customization

### Settings Search

- Use the search bar at the top of Settings to quickly find options
- Use quick navigation chips to jump between sections

### Custom Note Keys

`Settings → Note Keys`:

- Choose preset layouts:
  - **QWERTY**
  - **QWERTZ**
  - **AZERTY**
- Or fully customize:
  - Click any key to rebind
  - Perfect for non-standard layouts

### Custom Window Detection

If the game isn’t detected properly (e.g., through a launcher / different name):

1. Open `Settings → Window Detection`
2. Add window titles or process names
3. Click **Add**

Built-in detection includes:
- `Where Winds Meet`
- `WWM`
- `GeForce Now`
- `燕云十六声`
- `연운`

---

## 🎵 Band Mode (Experimental)

> ⚠️ **Experimental & unstable.**  
> Tested mainly on **local networks**. Expect bugs.

Play together with friends by splitting notes or tracks between multiple players.

### How it Works

1. One player creates a room (host)
2. Host shares the **6-character room code**
3. Friends join using the code
4. Host picks a song (auto-transfers to players who don't have it)
5. Each member clicks **Ready**
6. Host clicks **Play** for synchronized playback

### Play Modes

| Mode           | Description                                          |
|----------------|------------------------------------------------------|
| **Split Notes**| Notes distributed round-robin among players          |
| **By Track**   | Each player picks a MIDI track (e.g. melody, bass)   |

### Sync & Calibration

- Host can adjust sync delay from **-2s to +5s**
- **Test** mode: plays test notes to all members
- If others sound **ahead** of you → decrease delay  
- If others sound **behind** you → increase delay

### Relay Server (TURN)

- For strict NAT / firewalled networks:
  - Enable **Relay Server** before creating/joining
- Helps P2P connections succeed where direct connections fail

---

## 🖱 In-App Controls

- Click any song – start playing
- **Ctrl+click** – select/deselect songs
- **Shift+click** – select a range
- **Right-click** – rename / delete / open file location
- **Heart icon** – add/remove favorite
- **Playlist icon** – add to queue / playlist
- **Drag handle (top of sidebar)** – move window
- **Bottom bar**:
  - Play / Pause / Stop
  - Timeline: click/drag to seek
  - Loop toggle
  - Octave shift (+/-)
  - Note mode selector
  - Track selector
- **Mini mode button** – collapse to floating icon

---

## 📁 Managing Playlists

1. Open the **Playlists** tab
2. Click **New** to create a playlist
3. Name it and confirm
4. Add songs using the playlist icon in the library
5. Click a playlist to view and edit
6. **Drag** to reorder songs
7. Click **X** to remove from playlist
8. Click **Play** to load playlist into queue and start playing

---

## 💡 Tips

- **Finding MIDI files**:  
  Search `"song name midi"` or `"song name .mid"` online.
- **Bulk import**:  
  Use the **Import** button to load entire folders or ZIPs.
- **Song sounds wrong?**  
  - Try different **note modes** (`[` / `]`)
  - Adjust **octave shift** (+/-)
- **Too high / too low?**  
  Octave shift can fix pitch.
- **Continuous playback**:  
  Add multiple songs to the queue.
- **Quick access**:  
  Use the **search box** and **sort options**.
- **Mini mode**:  
  Press **Insert** to hide the full UI while playing.
- **Track selector**:  
  Use it to play only melody/lead instruments from a MIDI.

---

## 🛠 Troubleshooting

### Keys not registering in-game

- Make sure **Where Winds Meet** is running
- Game window must be **open** (can be in background, not minimized)
- You must be inside the **music minigame**

### Hotkeys not working

- Some keys may be used by other apps
- Browsers use **F12** for dev tools → try **End** instead
- Make sure the app is actually running (check system tray)

### Music sounds wrong

- The game only supports **21 keys (3 octaves)** – complex songs may be imperfect
- Try other **note modes**
- Use **octave shift** to move the song into a better range
- Try different MIDI versions of the song

### Songs not showing up

- `.mid` extension is required
- Files must be in the configured **album** folder
- Click **Refresh** in the sidebar to reload

### Progress bar jumps

- Can happen if multiple playback sources conflict
- Try **Stop → Play** again

---

## 📂 Where to Put MIDI Files

```text
wwm-overlay/
├── wwm-overlay.exe
├── album/              <- Put your .mid files here!
│   ├── song1.mid
│   ├── song2.mid
│   └── song3.mid
└── ...
```

## 🧪 Testing

- `npm run test` - run the Vitest suite once (global jest-dom helpers are preloaded via src/setupTests.js).
- `npm run test:watch` - keep Vitest in watch mode while you edit.
- `npm run test:coverage` - emit coverage reports (text + lcov.info) via the built-in v8 provider; the report now covers every JavaScript and Svelte file outside `node_modules/` and writes results under coverage/.
- `npm run coverage:check` - parse coverage/lcov.info and emit a warning if the line coverage for `src/lib/utils/**/*.js` and `src/lib/version.js` stays below 80% so the gate stays focused on the shared helper logic.

The coverage workflow (.github/workflows/coverage.yml) runs `npm run test:coverage` and `npm run coverage:check` on pushes to main and on pull requests so reviewers get soft warnings whenever coverage dips below the project-wide 80% target.

