use crate::ppu::frame::Frame;
use std::fs;

use bus::Bus;
use cartridge::{Mirroring, Rom};
use cpu::CPU;
use joypad::JoypadButton;
use ppu::{palette, PPU};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

fn main() {
    let path = std::env::args().nth(1).expect("no path given");

    // init sdl2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let width = 256;
    let height = 240;

    let window = video_subsystem
        .window("NEStor", width * 3 as u32, height * 3 as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    // canvas.set_scale(3.0, 3.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, width, height)
        .unwrap();

    let game_code = fs::read(path).expect("Should have been able to read the game");
    let rom = Rom::new(&game_code).unwrap();

    // let ppu_window = video_subsystem
    //     .window("PPU Viewer", 256 * 3, 128 * 3)
    //     .position_centered()
    //     .build()
    //     .unwrap();

    // let mut ppu_canvas = ppu_window.into_canvas().present_vsync().build().unwrap();

    // let ppu_creator = ppu_canvas.texture_creator();
    // let mut ppu_texture = ppu_creator
    //     .create_texture_target(PixelFormatEnum::RGB24, 256, 128 + 40)
    //     .unwrap();

    // let nametable_window = video_subsystem
    //     .window("Nametable Viewer", 800, 600)
    //     .position_centered()
    //     .build()
    //     .unwrap();

    // let mut nametable_canvas = nametable_window
    //     .into_canvas()
    //     .present_vsync()
    //     .build()
    //     .unwrap();

    // let nametable_creator = nametable_canvas.texture_creator();
    // let mut nametable_texture = nametable_creator
    //     .create_texture_target(PixelFormatEnum::RGB24, 512, 480)
    //     .unwrap();

    // the game cycle
    let bus = Bus::new(rom, move |ppu: &PPU, joypad: &mut joypad::Joypad| {
        texture.update(None, &ppu.frame.data, 256 * 3).unwrap();

        canvas.copy(&texture, None, None).unwrap();

        // render_tile_borders(&mut canvas);

        canvas.present();

        // let ppu_frame = ppu_viewer(ppu);
        // let palette_frame = palette_viewer(ppu);

        // ppu_texture
        //     .update(Rect::new(0, 0, 256, 128), &ppu_frame.data, 256 * 3)
        //     .unwrap();

        // ppu_texture
        //     .update(Rect::new(0, 136, 256, 8), &palette_frame.data, 256 * 3)
        //     .unwrap();

        // ppu_canvas.copy(&ppu_texture, None, None).unwrap();

        // ppu_canvas.present();

        // let nametable_frame = nametable_viewer(ppu);

        // nametable_texture
        //     .update(None, &nametable_frame.data, 512 * 3)
        //     .unwrap();

        // nametable_canvas
        //     .copy(&nametable_texture, None, None)
        //     .unwrap();
        // nametable_canvas.present();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),

                Event::KeyDown { keycode, .. } => {
                    if let Some(key) = get_joypad_button(keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_button_pressed_status(key, true);
                    }
                }
                Event::KeyUp { keycode, .. } => {
                    if let Some(key) = get_joypad_button(keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_button_pressed_status(key, false);
                    }
                }

                _ => { /* do nothing */ }
            }
        }
    });

    let mut cpu = CPU::new(bus);
    cpu.reset();

    loop {
        cpu.run_with_callback(|cpu| {
            // println!("{}", trace::trace(cpu));
        });
    }
}

pub fn render_tile_borders(canvas: &mut Canvas<Window>) {
    let scale = 3;
    canvas.set_draw_color(Color::RGB(200, 200, 200));

    for x in 0..32 {
        for y in 0..30 {
            let rect = Rect::new(
                8 * x * scale,
                8 * y * scale,
                8 * scale as u32,
                8 * scale as u32,
            );
            canvas.draw_rect(rect).unwrap();
        }
    }
}

fn get_joypad_button(keycode: Keycode) -> Option<JoypadButton> {
    match keycode {
        Keycode::Down => Some(JoypadButton::DOWN),
        Keycode::Up => Some(JoypadButton::UP),
        Keycode::Right => Some(JoypadButton::RIGHT),
        Keycode::Left => Some(JoypadButton::LEFT),
        Keycode::Space => Some(JoypadButton::SELECT),
        Keycode::Return => Some(JoypadButton::START),
        Keycode::A => Some(JoypadButton::BUTTON_A),
        Keycode::S => Some(JoypadButton::BUTTON_B),
        _ => None,
    }
}

fn ppu_viewer(ppu: &PPU) -> Frame {
    let mut frame = Frame::new(256, 128);
    let palette = &ppu.palette_table[0..4];

    for bank_idx in 0..2 {
        let offset = bank_idx * 128;
        let mut tile_y = 0;
        let mut tile_x = offset;

        let bank = (bank_idx * 0x1000) as usize;
        for tile_n in 0..256 {
            if tile_n != 0 && tile_n % 16 == 0 {
                tile_y += 8;
                tile_x = offset;
            }
            let tile =
                &ppu.rom.borrow_mut().chr_rom[(bank + tile_n * 16)..=(bank + tile_n * 16 + 15)];

            for y in 0..=7 {
                let mut upper = tile[y];
                let mut lower = tile[y + 8];

                for x in (0..=7).rev() {
                    let value = (1 & upper) << 1 | (1 & lower);
                    upper = upper >> 1;
                    lower = lower >> 1;

                    let rgb = match value {
                        0..=3 => palette::SYSTEM_PALETTE[palette[value as usize] as usize],
                        _ => panic!("can't be"),
                    };

                    frame.set_pixel(tile_x + x, tile_y + y, rgb)
                }
            }

            tile_x += 8;
        }
    }

    frame
}

fn palette_viewer(ppu: &PPU) -> Frame {
    let mut frame = Frame::new(256, 8);

    let mut tile_x = 0;
    for color in ppu.palette_table {
        for y in 0..8 {
            for x in 0..8 {
                frame.set_pixel(tile_x + x, y, palette::SYSTEM_PALETTE[color as usize]);
            }
        }
        tile_x += 8;
    }

    frame
}

fn bg_pallette(ppu: &PPU, attribute_table: &[u8], tile_x: usize, tile_y: usize) -> [u8; 4] {
    let group = tile_y / 4 * 8 + tile_x / 4;
    let attribute_byte = attribute_table[group];

    let shift = ((tile_y & 0x02) << 1) | (tile_x & 0x02);
    let palette_idx = (attribute_byte >> shift) & 0x03;

    let palette_base = (palette_idx as usize) * 4;

    [
        ppu.palette_table[0],
        ppu.palette_table[palette_base + 1],
        ppu.palette_table[palette_base + 2],
        ppu.palette_table[palette_base + 3],
    ]
}

fn nametable_viewer(ppu: &PPU) -> Frame {
    let mut frame = Frame::new(512, 480);
    let mut x_offset = 0;
    let mut y_offset = 0;

    for _ in 0..2 {
        for nametable in ppu.vram.chunks(0x400) {
            let attribute_table = &nametable[0x3c0..0x400];

            for i in 0..0x3c0 {
                let tile_index = nametable[i] as usize;
                let bank = ppu.ctrl.bknd_pattern_addr() as usize;

                let tile_block = &ppu.rom.borrow().chr_rom
                    [(bank + tile_index * 16)..(bank + tile_index * 16 + 16)];

                // println!("nt: {} tile_block: {}", tile_index, tile_block);

                let tile_x = i % 32;
                let tile_y = i / 32;

                let palette = bg_pallette(ppu, attribute_table, tile_x, tile_y);

                for y in 0..8 {
                    let mut lower = tile_block[y];
                    let mut upper = tile_block[y + 8];

                    for x in (0..8).rev() {
                        let value = (lower & 0x01) | ((upper & 0x01) << 1);

                        lower = lower >> 1;
                        upper = upper >> 1;

                        let rgb = match value {
                            0..=3 => palette::SYSTEM_PALETTE[palette[value as usize] as usize],
                            _ => panic!("can't be"),
                        };

                        let x = tile_x * 8 + x;
                        let y = tile_y * 8 + y;

                        frame.set_pixel(x_offset + x, y_offset + y, rgb);
                    }
                }
            }

            if ppu.rom.borrow().mirroring == Mirroring::Vertical {
                x_offset = 30 * 8;
            } else {
                y_offset = 30 * 8;
            }
        }

        if ppu.rom.borrow().mirroring == Mirroring::Vertical {
            x_offset = 0;
            y_offset = 30 * 8;
        } else {
            x_offset = 30 * 8;
            y_offset = 0;
        }
    }

    frame
}
