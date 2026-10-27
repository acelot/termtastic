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
- Optimized for small screens (down to 70x20 characters)
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

- [x] Storing messages in local DB
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
- [x] Ability to copy public key to clipboard
- [x] Ability to delete node feature
- [x] Telemetry info
- [x] Traceroute feature
- [x] Extended traceroute info popup
- [x] Ability to copy traceroute result
- [x] Storing traceroutes in the local DB
- [x] Position info
- [ ] Ignore node feature
- [ ] Add node to favorite feature

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

- [x] Battery indicator
- [x] DB write indicator
- [x] Traceroute indicator
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
| Bluetooth devices discovery | [btleplug](https://github.com/deviceplug/btleplug)                                                                        |
| TCP devices discovery       | [mdns-sd](https://github.com/keepsimple1/mdns-sd)                                                                    |
| Logging                     | [tracing](https://github.com/tokio-rs/tracing)                                                                       |
| Async/Channels              | [tokio](https://github.com/tokio-rs/tokio)                                                                           |
| Configuration               | [confy](https://github.com/rust-cli/confy), [etcetera](https://github.com/lunacookies/etcetera)                      |
| Errors                      | [anyhow](https://github.com/dtolnay/anyhow), [thiserror](https://github.com/dtolnay/thiserror)                       |
| Datetime                    | [chrono](https://github.com/chronotope/chrono)                                                                       |
| Emoji selector              | [emojis](https://github.com/rossmacarthur/emojis)                                                                         |
| Local DB                    | [rusqlite](https://github.com/rusqlite/rusqlite), [rusqlite_migration](https://github.com/cljoly/rusqlite_migration) |

## Compatibility

✅ – tested, 🔬 – untested, ❌ – not working

| Feature                      |  Linux   |  macOS   |  Windows   |
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
