pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod joypad;
pub mod mapper;
pub mod opcodes;
pub mod ppu;
pub mod render;
pub mod trace;

use std::collections::HashMap;
use std::fs;

use bus::Bus;
use cartridge::Rom;
use cpu::CPU;
use ppu::NesPPU;
use render::frame::Frame;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

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

    let mut key_map = HashMap::new();
    key_map.insert(Keycode::Down, joypad::JoypadButton::DOWN);
    key_map.insert(Keycode::Up, joypad::JoypadButton::UP);
    key_map.insert(Keycode::Right, joypad::JoypadButton::RIGHT);
    key_map.insert(Keycode::Left, joypad::JoypadButton::LEFT);
    key_map.insert(Keycode::Space, joypad::JoypadButton::SELECT);
    key_map.insert(Keycode::Return, joypad::JoypadButton::START);
    key_map.insert(Keycode::A, joypad::JoypadButton::BUTTON_A);
    key_map.insert(Keycode::S, joypad::JoypadButton::BUTTON_B);

    let game_code = fs::read(path).expect("Should have been able to read the game");
    let rom = Rom::new(&game_code).unwrap();

    // the game cycle
    let bus = Bus::new(rom, move |ppu: &NesPPU, joypad: &mut joypad::Joypad| {
        texture.update(None, &ppu.frame.data, 256 * 3).unwrap();

        canvas.copy(&texture, None, None).unwrap();

        // render_tile_borders(&mut canvas);

        canvas.present();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),

                Event::KeyDown { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_button_pressed_status(*key, true);
                    }
                }
                Event::KeyUp { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_button_pressed_status(*key, false);
                    }
                }

                _ => { /* do nothing */ }
            }
        }
    });

    let mut cpu = CPU::new(bus);
    cpu.reset();
    cpu.run_with_callback(|cpu| {
        // println!("{}", trace::trace(cpu));
    })
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
