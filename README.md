<!-- markdownlint-disable MD033 -->
<h1 align="center"><img src="./logo.png" width="466" alt="termtastic"></h1>

<p align="center">
  <b>termtastic</b> is a feature-rich handmade <a href="https://meshtastic.org">Meshtastic®</a> console client written in Rust.
</p>

<p align="center">
  <a href="https://meshtastic.org"><img src="./logo-mpowered.png" height="20" alt="M-Powered"/></a>
  <a href="https://github.com/acelot/termtastic/actions"><img src="https://github.com/acelot/termtastic/actions/workflows/build.yml/badge.svg?event=push" height="20" alt="Build"/></a>
  <img src="https://img.shields.io/github/languages/code-size/acelot/termtastic?cacheSeconds=86400" height="20" alt="code size"/>
  <img src="https://img.shields.io/badge/vibecode--free-pink" height="20" alt="vibecode-free"/>
  <a href="./LICENSE"><img src="https://img.shields.io/github/license/acelot/termtastic.svg?style=flat&v=2" height="20" alt="GPL-3.0"/></a>
</p>

<p align="center">
  <a title="This tool is Tool of The Week on Terminal Trove, The $HOME of all things in the terminal" href="https://terminaltrove.com/termtastic/" target="_blank">
    <kbd>
      <picture>
        <source media="(prefers-color-scheme: dark)" srcset="https://cdn.terminaltrove.com/media/badges/tool_of_the_week/svg/terminal_trove_tool_of_the_week_green_on_dark_grey_bg.svg">
        <img alt="Terminal Trove Tool of The Week" height="64" src="https://cdn.terminaltrove.com/media/badges/tool_of_the_week/svg/terminal_trove_tool_of_the_week_black_on_white_bg.svg">
      </picture>
    </kbd>
  </a>
</p>

## Screenshots

<table>
  <tr>
    <td><img width="774" height="548" alt="channels" src="https://github.com/user-attachments/assets/81eda371-dcc3-4e9c-8316-aca3fac95335" /></td>
    <td><img width="774" height="548" alt="chat" src="https://github.com/user-attachments/assets/516d0212-b797-4e30-857d-944626f7f0f7" /></td>
  </tr>
  <tr>
    <td><img width="774" height="548" alt="reactions" src="https://github.com/user-attachments/assets/7c257c98-c593-45d8-8d27-cf21a6825082" /></td>
    <td><img width="774" height="548" alt="emoji" src="https://github.com/user-attachments/assets/b7567185-2b01-4640-aa87-02cce8359d2e" /></td>
  </tr>
  <tr>
    <td><img width="774" height="548" alt="multiline" src="https://github.com/user-attachments/assets/69809b69-0cd0-4281-b41a-7a3ed7f0b153" /></td>
    <td><img width="774" height="548" alt="settings" src="https://github.com/user-attachments/assets/cf328adb-4028-4a0a-9fdb-de32dfaa2535" /></td>
  </tr>
  <tr>
    <td><img width="774" height="548" alt="nodes" src="https://github.com/user-attachments/assets/8016640f-1b3b-4b09-8632-b74b46397b6b" /></td>
    <td><img width="774" height="548" alt="filters" src="https://github.com/user-attachments/assets/990bd4bd-4fe7-4975-a381-f6da885be0ab" /></td>
  </tr>
  <tr>
    <td><img width="774" height="548" alt="node_info" src="https://github.com/user-attachments/assets/2871b8d5-6adb-4b95-a07f-aa08f6525f95" /></td>
    <td><img width="774" height="548" alt="traceroute_popup" src="https://github.com/user-attachments/assets/a9de448a-405c-421c-9d17-de7c21996396" /></td>
  </tr>
  <tr>
    <td><img width="774" height="548" alt="position" src="https://github.com/user-attachments/assets/9584f221-bc1f-42d1-a8f1-923eff2e0621" /></td>
    <td><img width="774" height="548" alt="telemetry" src="https://github.com/user-attachments/assets/76851de4-8589-471d-bab6-879ab376655e" /></td>
  </tr>
  <tr>
    <td><img width="774" height="548" alt="connection" src="https://github.com/user-attachments/assets/3bcae127-33c2-4d00-953b-855776b3744b" /></td>
    <td><img width="774" height="548" alt="device_discovery" src="https://github.com/user-attachments/assets/cb138dd9-ee56-4ce3-b3c5-26fa6f6613dc" /></td>
  </tr>
  <tr>
    <td><img width="774" height="548" alt="logs" src="https://github.com/user-attachments/assets/6a806f9b-be6b-4d34-ac29-48a4210305c8" /></td>
    <td><img width="774" height="548" alt="log_expanded" src="https://github.com/user-attachments/assets/cbd68ef7-4eca-4d13-9f80-ed8333f1e5b8" /></td>
  </tr>
</table>

| :warning: WARNING                                                                     |
|:--------------------------------------------------------------------------------------|
| The project is under active development; things may change completely without notice. |

## Tech highlights

- Fully handwritten code, no vibe-coding
- Dependency-free single-binary application
- Fully asynchronous (Tokio channel-based)
- Optimized for small screens (down to `70x20`)
- Respects the user's terminal color scheme (no opinionated TrueColor colors)
- Keyboard-centric UI navigation
- Minimal use of emoji and non-ASCII characters in the UI
- Small memory footprint (`~20KB`)
- Supports multiple connection protocols (TCP, BLE, serial)
- Can discover devices automatically: paired BLE devices, TCP devices via mDNS, serial devices
- Uses standard system directories for configuration, database and logs

## Features

- Chat list (channels and direct messages)
- Multiline message support
- Reply to messages
- Reactions with emoji
- Node list with filtering and sorting
- Node info popup with detailed information
- Traceroute
- Position
- Telemetry
- Settings (all available sections, including module configs)
- Connection manager with auto-discovery (TCP, BLE, serial)
- Logs screen
- Indicators (battery, DB, traceroute, RX, online nodes)

## Not implemented features

- [ ] Ignore node feature
- [ ] Add node to favorites
- [ ] Import configuration from a link
- [ ] Set fixed GPS position
- [ ] Set time from host
- [ ] Lazy loading of messages

## Stack

| Feature                     | Library                                                                                                              |
|:----------------------------|:---------------------------------------------------------------------------------------------------------------------|
| TUI: Framework              | [Ratatui](https://ratatui.rs)                                                                                        |
| TUI: Backend                | [crossterm](https://github.com/crossterm-rs/crossterm)                                                               |
| TUI: Inputs                 | [ratatui-textarea](https://github.com/ratatui/ratatui-textarea)                                                      |
| TUI: Lists                  | [tui-widget-list](https://github.com/preiter93/tui-widget-list)                                                      |
| Interaction with Meshtastic | [meshtastic](https://github.com/meshtastic/rust)                                                                     |
| Clipboard functionality     | [arboard](https://github.com/1Password/arboard)                                                                      |
| Bluetooth devices discovery | [btleplug](https://github.com/deviceplug/btleplug)                                                                   |
| TCP devices discovery       | [mdns-sd](https://github.com/keepsimple1/mdns-sd)                                                                    |
| Logging                     | [tracing](https://github.com/tokio-rs/tracing)                                                                       |
| Async/Channels              | [tokio](https://github.com/tokio-rs/tokio)                                                                           |
| Configuration               | [confy](https://github.com/rust-cli/confy), [etcetera](https://github.com/lunacookies/etcetera)                      |
| Errors                      | [anyhow](https://github.com/dtolnay/anyhow), [thiserror](https://github.com/dtolnay/thiserror)                       |
| Datetime                    | [chrono](https://github.com/chronotope/chrono)                                                                       |
| Emoji selector              | [emojis](https://github.com/rossmacarthur/emojis)                                                                    |
| Local DB                    | [rusqlite](https://github.com/rusqlite/rusqlite), [rusqlite_migration](https://github.com/cljoly/rusqlite_migration) |

## Compatibility

✅ – tested, 🔬 – untested, ❌ – not working

| Feature                      |  Linux   |  macOS   |  Windows   |
|:-----------------------------|:--------:|:--------:|:----------:|
| BLE devices discovery        |    ✅    |    ✅    |     ✅     |
| Serial devices discovery     |    ✅    |    ✅    |     ✅     |
| TCP devices discovery (mDNS) |    ✅    |    ✅    |     ✅     |
| Copy to clipboard            |    ✅    |    ✅    |     ✅     |

## Download

<table>
  <thead>
    <tr>
      <th align="left">Source</th>
      <th align="left">Link</th>
      <th align="left">Notes</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>Manual download</td>
      <td><a href="https://github.com/acelot/termtastic/releases">Releases</a></td>
      <td></td>
    </tr>
    <tr>
      <td>Debian APT</td>
      <td><em>TBA</em></td>
      <td></td>
    </tr>
    <tr>
      <td>Arch Linux AUR</td>
      <td><a href="https://aur.archlinux.org/packages?K=termtastic">termtastic · termtastic-bin · termtastic-git</a></td>
      <td>Maintained by <a href="https://aur.archlinux.org/account/Ateles">Ateles</a></td>
    </tr>
    <tr>
      <td>macOS Brew</td>
      <td><pre lang="sh">brew install acelot/tap/termtastic</pre></td>
      <td>Updated automatically on release</td>
    </tr>
    <tr>
      <td>Windows WinGet</td>
      <td><em>TBA</em></td>
      <td></td>
    </tr>
  </tbody>
</table>

## FAQ

### How to launch a manually downloaded app on macOS?

To run an unsigned application on macOS, you need to dequarantine it using the command below:

```sh
xattr -d com.apple.quarantine ./path/to/termtastic
```

### Application doesn't discover BLE devices on macOS

To use Bluetooth on macOS Big Sur (11) or later, you need to enable the Bluetooth permission for your terminal. You can
do it by going to **System Preferences** → **Security & Privacy** → **Privacy** → **Bluetooth**, clicking the '+'
button, and selecting `Terminal` (or `iTerm` or whichever terminal application you use).

### Why emojis are glitching/tearing on my terminal?

If you are using `foot` terminal try to add these lines into your `foot.ini` config:

```ini
[tweak]
grapheme-width-method=wcswidth
```

The same named option exists in other terminals too. Check your terminal docs.

### Why do some emoji appear as squares in Windows Terminal??

Unfortunately, Windows Terminal cannot display some compound emoji such as 1️⃣, 2️⃣, 3️⃣, etc.
There is an issue in Github: <https://github.com/microsoft/terminal/issues/9708>

<hr/>

<sub>Meshtastic® is a registered trademark of Meshtastic LLC. termtastic is an independent project and is not affiliated with Meshtastic LLC.</sub>
