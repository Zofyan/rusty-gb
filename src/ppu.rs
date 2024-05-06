use crate::bus::Bus;
use crate::fetcher::Fetcher;
use crate::output::Output;

const LY: u16 = 0xFF44;
const LYC: u16 = 0xFF45;

pub trait Ppu {
    fn execute(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &Box<dyn Output>) -> Box<dyn Ppu> {
        Box::new(OAMFetch{ticks: 0})
    }
}

pub(crate) struct OAMFetch {
    pub(crate) ticks: u16,
}

impl Ppu for OAMFetch {
    fn execute(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &Box<dyn Output>) -> Box<dyn Ppu> {
        match self.ticks {
            ..=39 => {
                Box::new(OAMFetch {ticks: self.ticks + 1})
            }
            _ => {
                Box::new(PixelTransfer { ticks: 0 })
            }
        }
    }
}

struct PixelTransfer {
    ticks: u16,
}

impl Ppu for PixelTransfer {
    fn execute(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &Box<dyn Output>) -> Box<dyn Ppu> {
        match self.ticks {
            ..=159 => {
                fetcher.tick(bus);
                let empty = bus.fifo.is_empty();
                match empty {
                    false => {
                        let pixel = *bus.fifo.first().unwrap();
                        bus.fifo.pop();
                        output.write_pixel(self.ticks, bus.get(LY) as u16, pixel);
                        Box::new(PixelTransfer { ticks: self.ticks + 1})
                    }
                    true => Box::new(PixelTransfer { ticks: self.ticks })
                }
            }
            _ => {
                Box::new(HBlank { ticks: 0 })
            }
        }
    }
}

struct HBlank {
    ticks: u16,
}

impl Ppu for HBlank {
    fn execute(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &Box<dyn Output>) -> Box<dyn Ppu> {
        match self.ticks {
            ..=455 => {
                Box::new(HBlank {ticks: self.ticks + 1})
            }
            _ => {
                bus.set(LY, bus.get(LY) + 1);
                if bus.get(LY) == bus.get(LYC) {
                    bus.set_int_request_lcd(true);
                }
                if bus.get(LY) == 144 {
                    bus.set_int_request_vblank(true);
                    Box::new(VBlank { ticks: 0 })
                } else {
                    Box::new(OAMFetch { ticks: 0 })
                }
            }
        }
    }
}

struct VBlank {
    ticks: u16,
}

impl Ppu for VBlank {
    fn execute(&mut self, bus: &mut Bus, fetcher: &mut Fetcher, output: &Box<dyn Output>) -> Box<dyn Ppu> {
        match self.ticks {
            ..=455 => {
                Box::new(VBlank {ticks: self.ticks + 1})
            }
            _ => {
                bus.set(LY, bus.get(LY) + 1);
                if bus.get(LY) == 153 {
                    bus.set(LY, 0);
                    Box::new(OAMFetch { ticks: 0 })
                } else {
                    Box::new(VBlank { ticks: 0 })
                }
            }
        }
    }
}

