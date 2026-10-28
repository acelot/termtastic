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
    <td><img width="888" height="713" alt="channels" src="https://github.com/user-attachments/assets/390f3269-a337-4ce6-b121-3bf582254532" /></td>
    <td><img width="888" height="713" alt="chat" src="https://github.com/user-attachments/assets/065028e5-e68d-4ee6-bc2e-855a83d4a3cc" /></td>
  </tr>
  <tr>
    <td><img width="888" height="713" alt="reactions" src="https://github.com/user-attachments/assets/1440367b-2b33-423d-9f71-66f2a41346c9" /></td>
    <td><img width="888" height="713" alt="emoji" src="https://github.com/user-attachments/assets/26205ea9-7fae-4274-932a-cccc349c8f2a" /></td>
  </tr>
  <tr>
    <td><img width="888" height="713" alt="multiline" src="https://github.com/user-attachments/assets/551ea4b3-e507-4b36-a537-0cf9fa524060" /></td>
    <td><img width="888" height="713" alt="settings" src="https://github.com/user-attachments/assets/2f30c653-1738-4855-a43f-51adf459e856" /></td>
  </tr>
  <tr>
    <td><img width="888" height="713" alt="nodes" src="https://github.com/user-attachments/assets/5375e057-b70d-412a-9def-083ff9c06887" /></td>
    <td><img width="888" height="713" alt="filters" src="https://github.com/user-attachments/assets/2dc54bea-d257-47a1-b054-6a5afcb5fc3a" /></td>
  </tr>
  <tr>
    <td><img width="888" height="713" alt="node_info" src="https://github.com/user-attachments/assets/cf145ec5-2cfb-423a-ac6c-a52605ad7abc" /></td>
    <td><img width="888" height="713" alt="traceroute_popup" src="https://github.com/user-attachments/assets/f0bd0e88-cd26-4804-835c-580d172caf12" /></td>
  </tr>
  <tr>
    <td><img width="888" height="713" alt="position" src="https://github.com/user-attachments/assets/e0dfaddb-9f23-4222-88b0-935f4872de0d" /></td>
    <td><img width="888" height="713" alt="telemetry" src="https://github.com/user-attachments/assets/c0c79957-57a2-41d1-96a0-bb218379b847" /></td>
  </tr>
  <tr>
    <td><img width="888" height="713" alt="connection" src="https://github.com/user-attachments/assets/4753e672-0c59-4537-a7e9-bd1efc437dab" /></td>
    <td><img width="888" height="713" alt="device_discovery" src="https://github.com/user-attachments/assets/ec1f78b5-f145-46ee-8bcf-688392de2a1a" /></td>
  </tr>
  <tr>
    <td><img width="888" height="713" alt="logs" src="https://github.com/user-attachments/assets/180a72eb-d0c4-429a-8165-17723f9f0350" /></td>
    <td><img width="888" height="713" alt="log_expanded" src="https://github.com/user-attachments/assets/8df3d9df-336f-4e45-b233-d950ef8c9adc" /></td>
  </tr>
</table>

| :warning: WARNING                                                                       |
|:----------------------------------------------------------------------------------------|
| Project is under active development, things could be changed completely without notice. |

## Tech highlights

- Vibecode-free fully handwritten code
- Dependency-free single-binary application
- Fully asynchronous (Tokio channels-based)
- Optimized for small screens (down to `70x20` characters)
- Respects the user terminal colors (no fancy opinionated TrueColor schemes)
- Keyboard-centric UI navigation
- Minimum use of emoji/non-ascii characters in UI
- Small memory footprint (`~20KB`)
- Supports multiple connection protocols (TCP, BLE, Serial)
- Could discover devices automatically: BLE paired devices, TCP devices via mDNS, Serial devices
- Uses standard system directories for storing configuration, database and logs

## Features

- Chat list (channels and direct)
- Multiline messages support
- Reply to message
- Reactions using emoji
- Nodes list with filtering and sorting
- Node info popup with detailed information
- Traceroute
- Position
- Telemetry
- Settings (all available sections including Module configs)
- Connection manager with auto-discovery (TCP, BLE, Serial)
- Logs screen
- Indicators (battery, DB, traceroute, RX, online nodes)

## Not implemented features

- [ ] Ignore node feature
- [ ] Add node to favorite feature
- [ ] Import configuration from the link
- [ ] Set fixed GPS position
- [ ] Set time from host
- [ ] Messages lazy-loading

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
      <td>
        <a href="https://aur.archlinux.org/packages/termtastic">termtastic</a>,
        <a href="https://aur.archlinux.org/packages/termtastic-bin">termtastic-bin</a>,
        <a href="https://aur.archlinux.org/packages/termtastic-git">termtastic-git</a>
      </td>
      <td>Maintained by <a href="https://github.com/4goettma">Ateles</a></td>
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
