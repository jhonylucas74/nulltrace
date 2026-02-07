# nulltrace-client

Linux-style desktop UI for the Nulltrace hacker game. Built with Tauri 2, React, and TypeScript.

## Features

- **Login screen** – Mock auth; enter any username and click Login to continue.
- **Desktop** – Top bar (polybar/waybar style) with clock and app launcher, wallpaper, window management.
- **Terminal** – Simulated terminal with mock commands: `ls`, `whoami`, `help`, `clear`, `neofetch`.

All data is mocked; no connection to nulltrace-core yet.

## Run in browser (no Tauri)

```bash
npm install
npm run dev
```

Open http://localhost:1420. You can develop and test the full UI without building the native app.

## Run as Tauri app (desktop)

Install [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS (e.g. on Ubuntu: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, etc.).

```bash
npm install
npm run tauri dev
```

## Build for production

```bash
npm run build
npm run tauri build
```

## Project structure

- `src/` – React app: screens (Login, Desktop), components (TopBar, Window, Terminal), contexts (Auth), lib (mockCommands).
- `src-tauri/` – Tauri 2 Rust backend (minimal; no game logic yet).
- `index.html` – Loads Inter and JetBrains Mono from Google Fonts.

Design system (ricing-style dark theme) and CSS variables are in `src/index.css`.
