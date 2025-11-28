# WWM Overlay - MIDI Music Player
A beautiful music player for Where Winds Meet that plays your MIDI files by automatically pressing the right keyboard keys in-game.

> **Note:** 36-key mode is currently unstable and experimental. We're looking for a better approach. For best results, use the default 21-key mode.
<img width="1180" height="620" alt="image" src="https://github.com/user-attachments/assets/8977b742-7f7d-47d9-b78f-d36ed677e3c5" />


https://github.com/user-attachments/assets/4d25e203-0e4f-4b0f-8dc4-e855ce5e6647

https://github.com/user-attachments/assets/5223ff30-a859-4433-84c0-bfb3d8a8ed46


### Mini Mode

Collapse the app to a small floating icon while playing. The icon glows green when music is playing. Press `Insert` to toggle, or click the minimize button in the sidebar.

<img width="64" height="89" alt="Mini mode icon" src="https://github.com/user-attachments/assets/f0de318f-6a1a-4e92-93c8-ba73b42d4d13" />



## What is this?

This app lets you play music in Where Winds Meet's music minigame! Just add your MIDI files, click play, and the app will automatically press the keyboard keys for you. It's like having an auto-play feature for the in-game instrument.

## Support

If you enjoy this app, consider supporting me on Ko-fi!

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/snowiy)

## Features

- **Beautiful Spotify-style interface** - Dark theme with smooth animations
- **Easy to use** - Just drag your MIDI files and click play
- **Auto-update** - Get notified when new versions are available with one-click update
- **Play without focus** - No need to focus the game window, play while doing other things (local mode only)
- **Cloud gaming support** - Works with GeForce Now, Xbox Cloud Gaming, etc. (requires focus)
- **Smart music selection** - Automatically adjusts notes to fit the game's instrument
- **Multiple note modes** - 7 different note calculation algorithms to choose from
- **21/36 key toggle** - Switch between 21-key (natural notes) and 36-key (with sharps/flats) modes
- **Real-time mode switching** - Change note mode during playback instantly
- **Speed control** - Adjust playback speed from 0.25x to 2x
- **Octave shift control** - Adjust pitch up or down by up to 2 octaves
- **Queue system** - Build your playlist and play songs in order
- **Favorites** - Mark your favorite songs for quick access, drag to reorder
- **Search & sort** - Find songs instantly and sort by name, date, or duration
- **Multiple playlists** - Create, rename, and manage custom playlists
- **Drag & drop reordering** - Reorder songs in queue, favorites, and playlists
- **Custom album location** - Choose where to load MIDI files from in Settings
- **Real-time progress** - See exactly where you are in the song
- **Seek support** - Click anywhere on the timeline to jump to that position
- **Global hotkeys** - Control playback from anywhere, even when the game is focused
- **Mini mode** - Collapse to a small floating icon while playing
- **Song duration display** - See how long each MIDI file is

## How to Use

### First Time Setup

1. **Download the app** - Get the latest release from the releases page
2. **Extract the files** - Unzip to any folder you like
3. **Add your MIDI files** - Place your `.mid` files in the `album` folder
4. **Run as Administrator** - Right-click `wwm-overlay.exe` and select "Run as administrator"

> **Important:** The app requires administrator privileges to send keyboard inputs to the game.

### Playing Music

1. **Open the game** - Launch Where Winds Meet and open the music minigame
2. **Select a song** - In the app, click on any song in your library
3. **Add to queue** - Click the playlist icon to add songs to your queue or playlists
4. **Play** - Click the play button (or press F9)
5. **Enjoy!** - The music plays automatically, no need to focus the game window

> **Tip:** You can browse the web, use other apps, or do anything else while the music plays - the app sends keys directly to the game window!

### Keyboard Shortcuts (Global Hotkeys)

These shortcuts work even when the game is focused:

| Key | Action |
|-----|--------|
| **F9** | Play / Pause |
| **F10** | Previous track |
| **F11** | Next track |
| **F12** | Stop |
| **End** | Stop (alternative) |
| **[** | Previous note mode |
| **]** | Next note mode |
| **Insert** | Toggle mini mode |

### Note Calculation Modes

The app offers 7 different algorithms for mapping MIDI notes to the game's keys:

| Mode | Description |
|------|-------------|
| **YueLyn** | Recommended mode - YueLyn's favorite play mode |
| **Closest** | Finds the closest available note (best for most songs) |
| **Quantize** | Snaps to exact scale notes only |
| **Transpose Only** | Direct mapping with octave shifting |
| **Pentatonic** | Maps to 5-note pentatonic scale (do-re-mi-so-la) |
| **Chromatic** | Detailed 12-semitone to 7-key mapping |
| **Raw** | Direct 1:1 mapping, no processing (MIDI note % 21) |

You can change modes in real-time during playback using the `[` and `]` keys or the mode selector in the bottom bar.

### Key Modes (21 vs 36 Keys)

| Mode | Description |
|------|-------------|
| **21 Keys** | Natural notes only (default, most stable) |
| **36 Keys** | Includes sharps/flats (⚠️ experimental, unstable) |

Toggle between key modes using the "21/36" button in the bottom bar. **36-key mode is currently unstable** - we're exploring better approaches for sharp/flat support.

### Cloud Gaming Mode

For cloud gaming services like GeForce Now, Xbox Cloud Gaming, etc., enable **Cloud Gaming Mode** in Settings.

| Mode | How it works | Background play |
|------|--------------|-----------------|
| **Local (default)** | PostMessage to game window | Yes |
| **Cloud Gaming** | SendInput (global keyboard) | No |

**Important warnings for Cloud Gaming Mode:**
- Uses SendInput which simulates real keyboard globally
- You MUST keep the cloud gaming window focused
- Don't type while playing - your keyboard inputs will interfere!
- Background playback is NOT possible in this mode

### In-App Controls

- **Click any song** to start playing
- **Heart icon** - Add/remove from favorites
- **Playlist icon** - Add to queue or saved playlists
- **Drag handle** (top of sidebar) - Move the window around
- **Play/Pause button** - Control playback at the bottom
- **Timeline** - Click to seek, drag to scrub through the song
- **Loop button** - Toggle repeat mode
- **Octave shift** (+/-) - Adjust pitch up or down (bottom bar)
- **Mode selector** - Quick access to note calculation modes (bottom bar)
- **Minimize button** - Collapse to mini mode (floating icon)

### Managing Playlists

1. Go to the **Playlists** tab in the sidebar
2. Click **New** to create a playlist
3. Name your playlist and click Create
4. Add songs from the library using the playlist icon
5. Click on a playlist to view and manage its songs
6. **Drag songs** to reorder them
7. **Click X** on a song to remove it from the playlist
8. **Click Play** to load the playlist to queue and start playing

### Tips

- **Finding MIDI files**: Search online for "song name midi" or "song name .mid"
- **Song not playing right?**: Try different note modes! Press `[` or `]` to cycle through modes while playing
- **Too high or too low?**: Use the octave shift controls (+/-) in the bottom bar to adjust pitch
- **Multiple songs**: Add multiple songs to your queue for a continuous playlist
- **Searching**: Use the search box to quickly find songs in your library
- **Sorting**: Click the sort button to sort by name (A-Z), date added, or duration
- **Favorites**: Click the heart icon to quickly access your favorite songs later
- **Mini mode**: Press `Insert` to collapse the app while playing, press again to expand

## Troubleshooting

**Keys not registering in-game**
- Make sure Where Winds Meet is running
- The game window must be open (can be in background, but not minimized)
- Make sure you're in the music minigame interface

**Hotkeys not working**
- Some hotkeys may conflict with other applications
- F12 is commonly used by browsers (dev tools) - try using End instead for stop
- Make sure the app is running (check system tray)

**Music sounds wrong**
- The game only has 21 keys (3 octaves), so some complex songs won't sound perfect
- Try different note modes by pressing `[` or `]` - some modes work better for certain songs
- Use octave shift (+/-) if the song sounds too high or too low
- Try different MIDI files to see what works best

**Songs not showing up**
- Make sure your MIDI files are in the `album` folder
- Files must have the `.mid` extension
- Click the refresh button in the sidebar to reload the list

**Progress bar jumps around**
- This can happen if multiple playback sources conflict
- Try stopping and restarting the song

## Where to Put MIDI Files

```
wwm-overlay/
├── wwm-overlay.exe
├── album/              <- Put your .mid files here!
│   ├── song1.mid
│   ├── song2.mid
│   └── song3.mid
└── ...
```

## Support

Having issues? Here are some things to try:

1. Restart the app
2. Make sure the game is running
3. Check that your MIDI files are valid
4. Try a different MIDI file to see if the issue is file-specific

## Building from Source

If you want to build the app yourself:

1. **Install dependencies**:
   - [Rust](https://www.rust-lang.org/tools/install)
   - [Node.js](https://nodejs.org/) or [Bun](https://bun.sh/)

2. **Clone the repository** and navigate to the folder

3. **Install packages**:
   ```bash
   npm install
   # or
   bun install
   ```

4. **Run in development mode**:
   ```bash
   npm run tauri-dev
   # or
   bun run tauri-dev
   ```

5. **Build for release**:
   ```bash
   npm run tauri-build
   # or
   bun run tauri-build
   ```

6. **Find the executable**: After building, the app will be in `src-tauri/target/release/`

7. **Create album folder**: Make sure to create an `album` folder next to the `.exe` file and add your MIDI files there

## Credits

Built with:
- Tauri (desktop app framework)
- Svelte (user interface)
- Rust (backend and MIDI processing)

Music icon: Material Design Icons

Created by YueLyn

---

Enjoy making music in Where Winds Meet!
