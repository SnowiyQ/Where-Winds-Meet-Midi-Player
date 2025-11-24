# WWM Overlay - MIDI Music Player

A beautiful music player for Where Winds Meet that plays your MIDI files by automatically pressing the right keyboard keys in-game.

## What is this?

This app lets you play music in Where Winds Meet's music minigame! Just add your MIDI files, click play, and the app will automatically press the keyboard keys for you. It's like having an auto-play feature for the in-game instrument.

## Features

- **Beautiful Spotify-style interface** - Dark theme with smooth animations
- **Easy to use** - Just drag your MIDI files and click play
- **Smart music selection** - Automatically adjusts notes to fit the game's instrument
- **Queue system** - Build your playlist and play songs in order
- **Global hotkeys** - Control playback even when the game is focused
- **Real-time progress** - See exactly where you are in the song

## How to Use

### First Time Setup

1. **Download the app** - Get the latest release from the releases page
2. **Extract the files** - Unzip to any folder you like
3. **Add your MIDI files** - Place your `.mid` files in the `album` folder
4. **Run the app** - Double-click `wwm-overlay.exe`

### Playing Music

1. **Open the game** - Launch Where Winds Meet and open the music minigame
2. **Select a song** - In the app, click on any song in your library
3. **Add to queue** - Click the + button to add songs to your queue
4. **Play** - Click the play button (or press Scroll Lock)
5. **Focus the game** - The app will automatically switch focus to the game
6. **Enjoy!** - The music will play automatically

### Controls

**In the app:**
- Click any song to start playing
- Use the play/pause button at the bottom
- Click the + button to add songs to queue
- Drag the top bar to move the window around

**Global hotkeys (work even when game is focused):**
- `Scroll Lock` - Play/Pause
- `End` - Stop playback
- `Ctrl + N` - Next song
- `Ctrl + P` - Previous song
- `Ctrl + L` - Toggle loop mode

### Tips

- **Finding MIDI files**: Search online for "song name midi" or "song name .mid"
- **Song not playing right?**: The app automatically adjusts the notes, but some complex songs might not work well with the game's limited keyboard layout
- **Multiple songs**: Add multiple songs to your queue for a continuous playlist
- **Searching**: Use the search box to quickly find songs in your library

## Troubleshooting

**The app won't focus the game**
- Make sure Where Winds Meet is running
- The game window must be visible (not minimized)
- Try clicking on the game first, then press play

**Music sounds wrong**
- The game only has 21 keys (3 octaves), so some complex songs won't sound perfect
- The app tries its best to fit the notes into the available range
- Try different MIDI files to see what works best

**Songs not showing up**
- Make sure your MIDI files are in the `album` folder
- Files must have the `.mid` extension
- Click the refresh button (top left) to reload the list

**App is slow to start**
- If you have many MIDI files, the app now loads them instantly
- Durations will show as 0:00 but this is normal

## Where to Put MIDI Files

```
wwm-overlay/
├── wwm-overlay.exe
├── album/              ← Put your .mid files here!
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

---

Enjoy making music in Where Winds Meet!
