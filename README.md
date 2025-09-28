#Silentwave's Alpine Audio Visualizer

Alpine is a real-time audio visualizer that creates stunning 3D tunnel effects synchronized to your audio input. Experience your music like never before with this immersive WebGL-based visualization engine.

![Alpine Visualizer Screenshot](https://i.imgur.com/92bvUXM.jpeg)

## System Audio Setup

To use Alpine with system sounds, you need to loophole system audio to an input channel on your PC. Here are the recommended methods:

**Windows:**
- Enable "Stereo Mix" in your sound settings
- Or use Voicemeeter/VB-Audio virtual cables

**macOS:**
- Use BlackHole or Loopback Audio for virtual audio routing

**Linux:**
- Use PulseAudio loopback modules or ALSA loop devices

## Installation & Setup

### Python Dependencies
```bash
python3 -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
pip install -r requirements.txt
```

### WASM Build (Optional - for low-end version)
```bash
# Prerequisites: rustup, wasm-pack
cd wasm/alpine_lowend
make build
```

The WASM build outputs to `static/wasm/pkg/`

## Running Alpine

Start the server:
```bash
python app.py
```

Open your browser and navigate to:
- **Low-end WASM version:** http://localhost:5050/alpine_lowend

## Features

- **Real-time audio visualization** with 3D tunnel effects
- **Multiple audio input sources:**
  - Microphone input
  - File upload (audio/video files)
  - Device selection for different audio sources
- **Immersive visual effects** with dynamic colors and animations
- **Responsive design** that adapts to your screen size
- **High-performance WASM version** for lower-end systems

## Live Demo

Experience Alpine live at: https://silentwave.cc/alpine

## Technical Details

- **Backend:** Python Flask server
- **Frontend:** HTML5 Canvas with JavaScript/WebGL effects
- **WASM:** Rust-based high-performance version
- **Audio Processing:** Web Audio API with real-time FFT analysis
- **Dependencies:** Flask 3.0.3

## Browser Compatibility

Alpine works best in modern browsers with WebGL and Web Audio API support:
- Chrome/Chromium 60+
- Firefox 55+
- Safari 11+
- Edge 79+

For HTTPS deployment on production servers for best audio capture compatibility.