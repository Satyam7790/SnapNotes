# SnapNotes

Want to give ur screenshot a context ,use SnapNotes . SnapNotes is a local-first macOS screenshot notebook. It watches a folder for screenshots, copies imports into its private app data folder, and stores workspace documents in SQLite.

## Requirements

- macOS 12 or later
- Rust stable and the macOS Xcode command line tools
- Node.js 20 or later with npm

## Run in development

```sh
npm install
npm run tauri dev
```

The first development launch compiles the Rust application and frontend. The default watched folder is your Desktop. Use **Settings → Screenshot folder → Choose folder** to change it. Quickspace uses **⌥⇧S** to open its floating capture window.

## Build the macOS app and disk image

```sh
npm install
npm run tauri build
```

The bundled `.app` and `.dmg` are written under `src-tauri/target/release/bundle/`.

## Local data

Workspace records and TipTap document JSON live in the Tauri application data directory, in `database.sqlite`. Imported image copies are stored in `assets/<workspace-id>/`. Originals are left in place. Deleting a workspace removes its managed copies along with the workspace.

The first access to a protected folder such as Desktop may require macOS Files & Folders access. If monitoring does not start, allow Quickspace access under **System Settings → Privacy & Security → Files and Folders**, then reopen the app.

## Included

- SQLite workspace library with search, titles, timestamps, and text previews
- Screenshot folder watcher with completed-file stability checks and duplicate-event suppression
- Floating capture window with preview, queue navigation, Finder drop target, and add-to-existing action
- TipTap rich text editing, headings, lists, links, code blocks, multiple local images, image insertion by toolbar or Finder drop, and debounced autosave
- Tray menu, global shortcut, settings window, and background operation when the main window closes
- Light and dark themes

## Current constraints

- Folder watching is non-recursive.
- The editor stores images at their original aspect ratio; drag handles for resizing are not included yet.
- A signed and notarized distribution build requires the developer's Apple signing credentials. Local development builds can run without them.
