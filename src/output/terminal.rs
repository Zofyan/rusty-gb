use once_cell::sync::OnceCell;

use std::fmt::Debug;
use std::io;
use std::io::{stdout, Stdout, Write};
use colored::{Colorize, CustomColor};
use ratatui::backend::CrosstermBackend;
use crate::output::Output;

static PALLETS: OnceCell<Vec<Vec<String>>> = OnceCell::new();
pub struct Terminal {
    palettes: Vec<Vec<String>>,
    term: ratatui::Terminal<CrosstermBackend<Stdout>>,
    pixels: Vec<Vec<Option<&'static String>>>,
    diagnostic_string: String,
}

impl Output for Terminal {
    fn write_pixel(&mut self, x: u16, y: u16, color: u8, pallette: bool, debug: u8) {
        let character = match pallette {
            false => &PALLETS.get().unwrap()[0][color as usize],
            true => &PALLETS.get().unwrap()[1][color as usize],
        };
        if x < 160 && y < 144 {
            self.pixels[y as usize][x as usize] = Option::from(character);
        }
    }
    fn refresh(&mut self) -> bool {
        self.term.set_cursor_position((0, 0)).unwrap();
        println!("{}", self.diagnostic_string);
        //self.term.set_cursor_position((0, 1)).unwrap();
        println!("{}", self.pixels.iter().map(|x| x.iter().map(|x| x.unwrap().to_string()).collect::<Vec<String>>().join("")).collect::<Vec<String>>().join("\n"));
        true
    }

    fn set_diagnostics(&mut self, diagnostics: String) {
        self.diagnostic_string = diagnostics;
    }
}

impl Terminal {
    pub fn new(size: f64) -> Self {
        PALLETS.set(vec![vec![". ".custom_color(CustomColor::new(0, 0, 0)).to_string(), ". ".custom_color(CustomColor::new(80, 80, 80)).to_string(), ". ".custom_color(CustomColor::new(160, 160, 160)).to_string(), ". ".custom_color(CustomColor::new(255, 255, 255)).to_string()], vec![". ".custom_color(CustomColor::new(255, 255, 255)).to_string(), ". ".custom_color(CustomColor::new(160, 160, 160)).to_string(), ". ".custom_color(CustomColor::new(80, 80, 80)).to_string(), ". ".custom_color(CustomColor::new(0, 0, 0)).to_string()],]);
        let mut terminal = Terminal {
            palettes: vec![],
            term: ratatui::DefaultTerminal::new(CrosstermBackend::new(stdout())).expect("TODO: panic message"),
            pixels: vec![vec![Option::from(&PALLETS.get().unwrap()[0][0]); 160]; 144],
            diagnostic_string: "".to_string(),
        };
        terminal.term.clear();
        terminal
    }
}
