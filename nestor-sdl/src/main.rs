use std::path::Path;

use nestor::JoypadButton;
use nestor::NES;

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

    let mut nes = NES::new();

    nes.load_rom(path);
    nes.start_emulation();

    'running: loop {
        let frame = nes.emulate_frame();

        if let Some(frame) = frame {
            texture.update(None, &frame.data, 256 * 3).unwrap();

            canvas.copy(&texture, None, None).unwrap();

            canvas.present();

            // let nametable_frame = nes.nametable_viewer();

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
                    } => break 'running,

                    Event::KeyDown { keycode, .. } => {
                        if let Some(key) = get_joypad_button(keycode.unwrap_or(Keycode::Ampersand))
                        {
                            nes.cpu.bus.joypad1.set_button_pressed_status(key, true);
                        }
                    }
                    Event::KeyUp { keycode, .. } => {
                        if let Some(key) = get_joypad_button(keycode.unwrap_or(Keycode::Ampersand))
                        {
                            nes.cpu.bus.joypad1.set_button_pressed_status(key, false);
                        }
                    }
                    _ => { /* do nothing */ }
                }
            }
        }
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
