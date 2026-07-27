use std::fmt::Debug;
use std::time::Duration;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    window::WindowBuilder,
};
use winit::event::*;
use winit::event::Event;
use winit::event::WindowEvent;
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use winit::window::Window;
use crate::output::{Output, PX_COLOR, PX_PALETTE, SCREEN_HEIGHT, SCREEN_WIDTH};

/// RGBA for each `colour | palette` combination, pre-multiplied so the hot loop
/// is a 4-byte copy instead of the float multiply this used to do per pixel.
const RGBA: [[u8; 4]; 8] = [
    [0, 0, 0, 255],
    [25, 33, 25, 255],
    [50, 66, 50, 255],
    [75, 99, 75, 255],
    // palette bit set: the ramp is reversed
    [75, 99, 75, 255],
    [50, 66, 50, 255],
    [25, 33, 25, 255],
    [0, 0, 0, 255],
];

pub struct LCD {
    size: u32,
    pixels: Pixels<'static>,
    window: &'static Window,
    event_loop: EventLoop<()>,
}
impl Output for LCD {
    fn write_line(&mut self, y: u16, line: &[u8; SCREEN_WIDTH]) {
        if y as usize >= SCREEN_HEIGHT {
            return;
        }
        let frame: &mut [u8] = self.pixels.frame_mut();
        let row = &mut frame[y as usize * SCREEN_WIDTH * 4..][..SCREEN_WIDTH * 4];
        for (px, out) in line.iter().zip(row.chunks_exact_mut(4)) {
            out.copy_from_slice(&RGBA[(px & (PX_COLOR | PX_PALETTE)) as usize]);
        }
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
