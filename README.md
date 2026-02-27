# SillyTracker
A lightweight music player for tracking module formats, with a special focus on .mod files and other classic formats.

## Features
- 🎵 Multi-format support - Plays standard audio formats plus classic tracker modules
- 🎚️ MOD playback - Full support for Protracker / Soundtracker .mod files
- ⏯️ Playback controls - Play, pause, stop, and seek through tracks
- 📋 Playlist support - Queue up your favorite modules

## Supported Formats
- ProTracker	.mod	Classic Amiga module format
- MP3	.mp3	Standard audio
- WAV	.wav	Uncompressed audio
- OGG	.ogg	Compressed audio
- FLAC	.flac	Lossless audio

## Installation
Clone the repository
```bash
git clone https://github.com/purpflow1/sillytracker.git
```
Navigate to directory
```bash
cd sillytracker
```
Build the player
```bash
cargo build
```
Build for production
```bash
cargo build --release
```
The executable will be in the `/target/release` folder

## Contributing

Found a bug? Want to add support for another tracker format? PRs are welcome!

Fork the repository
1. Create your feature branch (git checkout -b feature/amazing-feature)
2. Commit your changes (git commit -m 'Add some amazing feature')
3. Push to the branch (git push origin feature/amazing-feature)
4. Open a Pull Request

## License
MIT License - see LICENSE file for details
