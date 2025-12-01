# WWM Overlay - MIDI Music Player
A beautiful music player for Where Winds Meet that plays your MIDI files by automatically pressing the right keyboard keys in-game.

> **Note:** 36-key mode uses instant Shift/Ctrl key combos for sharps and flats. If notes are dropping, try increasing the modifier delay in Settings.
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
- **Multiple note modes** - 9 different note calculation algorithms to choose from
- **21/36 key toggle** - Switch between 21-key (natural notes) and 36-key (with sharps/flats) modes
- **Real-time mode switching** - Change note mode during playback instantly
- **Track selector** - Choose specific MIDI tracks to play in solo mode
- **Speed control** - Adjust playback speed from 0.25x to 2x
- **Octave shift control** - Adjust pitch up or down by up to 2 octaves
- **Queue system** - Build your playlist and play songs in order
- **Favorites** - Mark your favorite songs for quick access, drag to reorder
- **Search & sort** - Find songs instantly and sort by name, duration, BPM, or difficulty
- **Multiple playlists** - Create, rename, and manage custom playlists
- **Drag & drop reordering** - Reorder songs in queue, favorites, and playlists
- **Custom album location** - Choose where to load MIDI files from in Settings
- **Real-time progress** - See exactly where you are in the song
- **Seek support** - Click anywhere on the timeline to jump to that position
- **Global hotkeys** - Control playback from anywhere, even when the game is focused
- **Mini mode** - Collapse to a small floating icon while playing
- **Song info display** - See BPM and difficulty (Easy/Medium/Hard/Expert) for each song
- **Quick favorite** - One-click favorite button for the currently playing song
- **Remember window position** - App remembers where you placed it
- **Large library support** - Optimized metadata caching for 10,000+ MIDI files
- **Band Mode** - Play together with friends (⚠️ experimental)
- **Custom keyboard layouts** - Supports QWERTY, QWERTZ, AZERTY presets or fully custom key bindings
- **Song Library (P2P)** - Share and download songs from other users online
- **Library management** - Right-click songs to rename, delete, or open file location
- **Custom window detection** - Add custom process names for game window detection
- **Settings search** - Quick search and navigation within settings
- **Remember band name** - Your band mode name is saved across sessions

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

The app offers 9 different algorithms for mapping MIDI notes to the game's keys:

| Mode | Description |
|------|-------------|
| **YueLyn** | Recommended mode - YueLyn's favorite play mode |
| **Closest** | Finds the closest available note (best for most songs) |
| **Wide** | Uses high and low rows more often (spreads notes across all octaves) |
| **Sharps** | 36-key mode: uses more Shift/Ctrl modifiers (shifts notes to sharps) |
| **Quantize** | Snaps to exact scale notes only |
| **Transpose Only** | Direct mapping with octave shifting |
| **Pentatonic** | Maps to 5-note pentatonic scale (do-re-mi-so-la) |
| **Chromatic** | Detailed 12-semitone to 7-key mapping |
| **Raw** | Direct 1:1 mapping, no processing (MIDI note % 21) |

You can change modes in real-time during playback using the `[` and `]` keys or the mode selector in the bottom bar.

### Key Modes (21 vs 36 Keys)

| Mode | Description |
|------|-------------|
| **21 Keys** | Natural notes only (default) |
| **36 Keys** | Includes sharps/flats using Shift/Ctrl modifiers |

Toggle between key modes using the "21/36" button in the bottom bar. 36-key mode sends modifier combos instantly (Shift+X, Ctrl+X) for sharps and flats.

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

### Song Library (P2P Sharing)

Share and discover MIDI files with other users! The Song Library uses peer-to-peer technology to share songs directly between users.

**How to use:**
1. Go to the **Share** tab in the Online section
2. Toggle **Enable Sharing** to connect
3. Your songs are automatically shared with other users
4. Browse songs from other users and click to download
5. Downloaded songs are added to your library

**Features:**
- See how many songs are available from other users
- Songs you already own are marked as "Owned"
- Download progress shown in real-time
- Auto-connects on app restart if previously enabled
- Custom discovery server URL (advanced users)

**Privacy notes:**
- Only file names and hashes are shared, not file contents until downloaded
- Songs are transferred directly between users (P2P)
- No account required

### Library Management

Right-click on any song in your library to access management options:
- **Rename** - Change the song file name
- **Delete** - Remove the song (with confirmation)
- **Open Location** - Open the folder containing the file

### Settings & Customization

**Settings Search**: Use the search bar at the top of Settings to quickly find options. Click the quick navigation buttons to jump to specific sections.

**Custom Note Keys**: Customize which keys play each note in Settings > Note Keys. Choose from preset layouts (QWERTY, QWERTZ, AZERTY) or create your own custom mapping. Click any key to rebind it - perfect for any keyboard layout!

**Custom Window Detection**: If the app doesn't detect your game window (e.g., running through a launcher or with a different name), add custom window titles in Settings:
1. Go to **Settings** > **Window Detection**
2. Enter the window title or process name
3. Click **Add**

Built-in detection includes: Where Winds Meet, WWM, GeForce Now, 燕云十六声, 연운

### Band Mode (Experimental)

> ⚠️ **Warning:** Band mode is experimental and unstable. Only tested on local networks - bugs are expected!

Play music together with friends! Each player handles different notes or tracks.

**How to use:**
1. One player creates a room (becomes host)
2. Share the 6-character room code with friends
3. Friends join using the code
4. Host selects a song (auto-transfers to members who don't have it)
5. Members click "Ready" when ready to play
6. Host clicks "Play" to start synchronized playback

**Play modes:**
| Mode | Description |
|------|-------------|
| **Split Notes** | Notes are automatically distributed among players (round-robin) |
| **By Track** | Each player picks a specific MIDI track to play |

**Sync Delay & Calibration:**
- Host can adjust delay (-2s to +5s) to compensate for network latency
- Click **Test** to start calibration mode - plays test notes on all members
- Adjust the slider until all players sound in sync
- If members play **ahead** of you → decrease the delay
- If members play **behind** you → increase the delay

**Known limitations:**
- Only tested on local networks
- P2P connection may fail on some network configurations
- Sync accuracy depends on network conditions
- File transfer only works for members who don't already have the file

### In-App Controls

- **Click any song** to start playing
- **Right-click any song** - Rename, delete, or open file location
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
- **Track selector**: Use the track dropdown in the bottom bar to play only specific instruments/tracks from a MIDI file

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
