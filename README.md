<p align="center"><img src="./logo.png" width="466" alt="termtastic"></p>

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

<table>
  <tr>
    <td><img width="886" height="713" alt="messenger" src="https://github.com/user-attachments/assets/8623b06a-ff40-443d-9c94-4f89cf30aca4" /></td>
    <td><img width="930" height="757" alt="messenger_emoji_selector" src="https://github.com/user-attachments/assets/f79e00b3-8675-4528-a8b0-3a065cdce7af" /></td>
  </tr>
  <tr>
    <td><img width="930" height="757" alt="messenger_reactions_viewer" src="https://github.com/user-attachments/assets/3887d587-0b3c-4fda-b8aa-deaf85219e5e" /></td>
    <td><img width="930" height="757" alt="messenger_multiline_message" src="https://github.com/user-attachments/assets/f3468114-0d09-438b-bbb8-f64e5fd6ee37" /></td>
  </tr>
  <tr>
    <td><img width="930" height="757" alt="channels" src="https://github.com/user-attachments/assets/fb94c88d-8c55-4359-af63-7ee8ca43a80c" /></td>
    <td><img width="930" height="757" alt="nodes" src="https://github.com/user-attachments/assets/4539b999-8f38-4fa5-9e84-740978856bad" /></td>
  </tr>
  <tr>
    <td><img width="930" height="757" alt="nodes_help" src="https://github.com/user-attachments/assets/bd8c6e66-c46a-49df-a2e7-7aef586e9fd0" /></td>
    <td><img width="930" height="757" alt="nodes_nodeinfo" src="https://github.com/user-attachments/assets/aaf27b86-8969-4ae7-a4e4-d02491988baa" /></td>
  </tr>
  <tr>
    <td><img width="930" height="757" alt="settings" src="https://github.com/user-attachments/assets/caf9da10-9e0c-4cb0-94a6-8d8eba66e4dd" /></td>
    <td><img width="930" height="757" alt="settings_bitmask_input" src="https://github.com/user-attachments/assets/5ac5494a-9e93-46ef-9a1d-3a4a323f7b54" /></td>
  </tr>
  <tr>
    <td><img width="930" height="757" alt="connections" src="https://github.com/user-attachments/assets/c09f3001-94e6-43d5-a92f-2c23b253eed5" /></td>
    <td><img width="930" height="757" alt="connections_discovery" src="https://github.com/user-attachments/assets/0f20de05-a1f2-4037-b179-da541f73b7b0" /></td>
  </tr>
  <tr>
    <td><img width="930" height="757" alt="logs" src="https://github.com/user-attachments/assets/942eaf5e-6771-4423-aff9-4e0b8dcf11fa" /></td>
    <td><img width="930" height="757" alt="logs_expanded_view" src="https://github.com/user-attachments/assets/f4faeb55-6b39-402b-8944-6a6f8d0b199e" /></td>
  </tr>
</table>

| :warning: WARNING                                                                       |
|:----------------------------------------------------------------------------------------|
| Project is under active development, things could be changed completely without notice. |

## Tech highlights

- Vibecode-free fully handwritten code
- Dependency-free single-binary application
- Fully asynchronous (Tokio channels-based)
- Optimized for small screens (down to 80x24 characters)
- Respects the user terminal colors (no fancy opinionated TrueColor schemes)
- Keyboard-centric UI navigation
- Minimum use of emoji/non-ascii characters in UI
- Small memory footprint (~20KB)
- Supports multiple connection protocols (TCP, BLE, Serial)
- Could discover devices automatically: BLE paired devices, TCP devices via mDNS, Serial devices
- Uses standard system directories for storing configuration, database and logs

## Features

> [!NOTE]  
> Unchecked items are not implemented yet.

### Chat tab

#### Channels

- [x] Scrollable channels list (Primary, Secondary)
- [x] Direct conversations
- [x] Display the last message for each channel
- [x] Purge channel chat feature

#### Messenger

- [ ] Storing messages in local DB
- [x] Scrollable chat screen
- [x] Display the short and long names of node
- [x] Display the SNR/RSSI for direct nodes
- [x] Display the number of hops for retranslated messages
- [x] Display the time of messages
- [x] Display the reactions (emojis)
- [x] Display the routing info for the retranslated messages and reactions
- [x] Ability to see detailed info about reactions
- [x] Ability to see node info of the selected message author
- [x] Ability to send broadcast messages to the channels
- [x] Ability to send direct messages to the nodes
- [x] Ability to reply to the messages
- [x] Ability to send the reactions (emojis)
- [x] Limiting the message length to 200 chars (with counter)

### Nodes tab

#### Nodes list

- [x] Storing nodes in local DB
- [x] Scrollable nodes list
- [x] Ability to start direct conversation with the selected node
- [x] Display the short and long names of node
- [x] Display the SNR/RSSI for direct nodes
- [x] Display the number of hops for the routed nodes
- [x] Display the ID of the nodes
- [x] Display the humanized last heard time of the nodes
- [x] Sort nodes by different fields: last heard, hop count, distance, etc.
- [x] Nodes fuzzy search
- [x] Nodes filtering by special tokens
- [x] Ability to see node info of the selected node

#### Single node expanded view

- [x] Display node detailed info
- [x] Copy public key to clipboard
- [x] Delete node feature
- [x] Telemetry info
- [x] Traceroute feature
- [x] Extended traceroute info popup
- [ ] Storing traceroutes in the local DB
- [ ] Ignore feature
- [ ] Add to Favorite feature
- [ ] Position info
- [ ] TBD

### Settings tab

#### General

- [x] Loading device configuration (generic feature)
- [x] Saving device configuration (generic feature)
- [x] Storing telemetry in the local DB
- [x] Loading last telemetry data for each node from the local DB on app start
- [ ] Import configuration from the link

#### App

- [x] UI

#### Radio

- [x] LoRa
- [x] Channels
- [x] Security

#### Device

- [x] User
- [x] Device
- [x] Position
- [x] Power
- [x] Display
- [x] Bluetooth
- [x] Network

#### Module

- [x] MQTT
- [x] Serial
- [x] External Notification
- [x] Store & Forward
- [x] Range Test
- [x] Telemetry
- [x] Canned Message
- [x] Neighbor Info
- [x] Ambient Lighting
- [x] Detection Sensor

### Connection tab

- [x] Scrollable devices list (TCP, BLE, Serial)
- [x] Connection via TCP
- [x] Connection via BLE
- [x] Connection via Serial
- [x] Device configuration loading during the connection process and storing it into state
- [x] Storing TCP connections into a config file
- [x] Discovering of BLE and Serial devices feature
- [x] Discovering TCP devices through mDNS
- [x] Reconnection feature with exponential backoff timeouts
- [x] Ability to set a name for the device

### Logs tab

- [x] Writing logs into files using a daily rolling strategy
- [x] Mirroring logs into the log list with scroll
- [x] Ability to expand the single log record (useful for long logs)
- [x] Ability to copy log record into clipboard

### General features

- [x] RX indicator
- [x] Online/Total nodes counter

## Stack

| Feature                     | Library                                                                                                              |
|:----------------------------|:---------------------------------------------------------------------------------------------------------------------|
| TUI: Framework              | [Ratatui](https://ratatui.rs)                                                                                        |
| TUI: Backend                | [crossterm](https://github.com/crossterm-rs/crossterm)                                                               |
| TUI: Inputs                 | [ratatui-textarea](https://github.com/ratatui/ratatui-textarea)                                                      |
| TUI: Lists                  | [tui-widget-list](https://github.com/preiter93/tui-widget-list)                                                      |
| Interaction with Meshtastic | [meshtastic](https://github.com/meshtastic/rust)                                                                     |
| Clipboard functionality     | [arboard](https://github.com/1Password/arboard)                                                                      |
| Bluetooth devices discovery | [bluest](https://github.com/alexmoon/bluest/)                                                                        |
| TCP devices discovery       | [mdns-sd](https://github.com/keepsimple1/mdns-sd)                                                                    |
| Logging                     | [tracing](https://github.com/tokio-rs/tracing)                                                                       |
| Async/Channels              | [tokio](https://github.com/tokio-rs/tokio)                                                                           |
| Configuration               | [confy](https://github.com/rust-cli/confy), [etcetera](https://github.com/lunacookies/etcetera)                      |
| Errors                      | [anyhow](https://github.com/dtolnay/anyhow), [thiserror](https://github.com/dtolnay/thiserror)                       |
| Datetime                    | [chrono](https://github.com/chronotope/chrono)                                                                       |
| Emoji selector              | [emoji](https://github.com/Shizcow/emoji-rs)                                                                         |
| Local DB                    | [rusqlite](https://github.com/rusqlite/rusqlite), [rusqlite_migration](https://github.com/cljoly/rusqlite_migration) |

## Compatibility

✅ – tested, 🔬 – untested, ❌ – not working

| Feature                      | 🐧 Linux | 🍏 macOS | 🪟 Windows |
|:-----------------------------|:--------:|:--------:|:----------:|
| BLE devices discovery        |    ✅     |    ✅     |     ✅      |
| Serial devices discovery     |    ✅     |    ✅     |     ✅      |
| TCP devices discovery (mDNS) |    ✅     |    ✅     |     ✅      |
| Copy to clipboard            |    ✅     |    ✅     |     ✅      |

## Download

| Source             | Link                                                      |
|:-------------------|:----------------------------------------------------------|
| Manual download    | [Releases](https://github.com/acelot/termtastic/releases) |
| Debian PPA         | 🏗️ TBA                                                   |
| Arch Linux AUR     | 🏗️ TBA                                                   |
| macOS Brew         | 🏗️ TBA                                                   |
| Windows Chocolatey | 🏗️ TBA                                                   |

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
