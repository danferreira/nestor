## NEStor

NEStor is a NES emulator crafted in Rust for educational and nostalgia purposes. It's not ready for daily gaming sessions yet.


### Keyboard to Gamepad Mapping

| Keyboard       | Gamepad           |
| -------------- | ----------------- |
| A              | A                 |
| S              | B                 |
| Space          | Select            |
| Enter          | Start             |
| Arrow Keys     | Directional Pad   |

### Emulator Shortcuts

| Keyboard       | Action                    |
| -------------- | ------------------------- |
| O              | Load a new ROM            |
| N              | Open Nametable Viewer     |
| P              | Open PPU Viewer           |


### TODO

- [x] CPU
    - [x] Official Instructions
    - [x] Unnoficial Instructions
    - [x] Addressing Modes
    - [x] Interrupts
- [ ] ROM
    - [x] Load rom
    - [ ] Mappers
        - [x] NROM
        - [x] CNROM
        - [ ] MMC1
        - [ ] UxROM
        - [ ] MMC3
- [ ] PPU
    - [x] Registers
    - [x] Loopy Registers
    - [x] Rendering
    - [x] Scrolling
    - [x] Sprite priority
    - [x] Sprite 0
    - [ ] Regions
        - [x] NTSC
        - [ ] PAL
        - [ ] Dendy
- [x] Gamepad
    - [x] 1p
    - [x] 2p
- [ ] APU
- [ ] Save/Load state support
- [ ] Frontends
    - [x] Desktop
        - [ ] Gui
            - [ ] Proper menus
            - [ ] Settings
                - [ ] Video config
                - [ ] Gamepad config
        - [ ] Debugger
            - [x] PPU Viewer
            - [x] Nametable Viewer
            - [ ] FPS display
            - [ ] Improve tracer
            - [ ] Disassembler

    - [ ] Browser (WASM)
        - [ ] Gui
            - [ ] Proper menus
            - [ ] Settings
                - [ ] Video config
                - [ ] Gamepad config
        - [ ] Debugger
            - [x] PPU Viewer
            - [x] Nametable Viewer
            - [x] FPS display
            - [ ] Improve tracer
            - [ ] Disassembler
