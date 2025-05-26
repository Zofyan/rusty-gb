use miniquad::*;
use macroquad::prelude::*;
use std::fmt::Debug;
use std::process::exit;
use std::time::Duration;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::WindowBuilder,
};
use winit::event::*;
use winit::event_loop::ControlFlow;
use winit::event::Event;
use winit::event::WindowEvent;
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::Window;
use crate::output::Output;

pub struct LCD {
    size: u32,
    pixels: Pixels<'static>,
    window: &'static Window,
    event_loop: EventLoop<()>,
}
impl Output for LCD {
    fn write_pixel(&mut self, x: u16, y: u16, color: u8, pallette: bool, _: u8) {
        if x >= 160 || y >= 144 {
            return;
        }
        let colors = match pallette {
            false => [0, 25, 50, 75],
            true => [75, 50, 25, 0],
        };
        let c = colors[color as usize];

        let x = x as usize;
        let y = y as usize;
        let frame: &mut [u8] = self.pixels.frame_mut();
        frame[x * 4 + y * 4 * 160 + 0] = c;
        frame[x * 4 + y * 4 * 160 + 1] = (c as f64 * 1.33) as u8;
        frame[x * 4 + y * 4 * 160 + 2] = c;
        frame[x * 4 + y * 4 * 160 + 3] = 255;
    }

    fn refresh(&mut self) -> bool {
        let timeout = Some(Duration::from_millis(0));

        let status = self.event_loop.pump_events(timeout, |event, elwt| {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == self.window.id() => elwt.exit(),
                Event::AboutToWait => {
                    self.window.request_redraw();
                }
                _ => (),
            }
        });
        if let PumpStatus::Exit(_exit_code) = status {
            return false
        }
        self.pixels.render().unwrap();
        true
    }
}

impl LCD {
    pub fn new(scale: u32) -> Self {
        let event_loop = EventLoop::new().unwrap();
        let window = Box::leak(Box::new(WindowBuilder::new()
            .with_title("Emulator")
            .with_inner_size(LogicalSize::new(160.0 * scale as f64, 144.0 * scale as f64))
            .build(&event_loop)
            .unwrap()));

        let surface_texture = SurfaceTexture::new(160 * scale * 2, 144 * scale * 2, &*window);
        let pixels = Pixels::new(160 * 1, 144 * 1, surface_texture).unwrap();
        LCD {
            size: scale,
            pixels,
            window,
            event_loop
        }
    }
}
