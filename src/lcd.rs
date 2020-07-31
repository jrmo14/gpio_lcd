use std::thread::{sleep, JoinHandle};
use std::time::{Duration, Instant, SystemTime, SystemTimeError};

use crate::icons::Icon;
use gpio_cdev::errors::Error;
use gpio_cdev::*;
use parking_lot::Mutex;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::ops::DerefMut;
use std::sync::{mpsc, Arc};
use std::thread;
use unidecode::unidecode;
// TODO add independent row scrolling and custom characters

// Adapted from Arduino standard library LiquidCrystal.cpp/h

// Commands
const LCD_CLEAR_DISPLAY: u8 = 0x01;
const LCD_RETURN_HOME: u8 = 0x02;
const LCD_ENTRY_MODE_SET: u8 = 0x04;
const LCD_DISPLAY_CONTROL: u8 = 0x08;
const LCD_CURSOR_SHIFT: u8 = 0x10;
const LCD_FUNCTION_SET: u8 = 0x20;
const LCD_SET_CGRAM_ADDR: u8 = 0x40;
const LCD_SET_DDRAM_ADDR: u8 = 0x80;

// Display entry mode
const LCD_ENTRY_LEFT: u8 = 0x02;
const LCD_ENTRY_SHIFT_DECREMENT: u8 = 0x00;

// Display on/off control
const LCD_DISPLAY_ON: u8 = 0x04;
const LCD_CURSOR_OFF: u8 = 0x00;
const LCD_BLINK_OFF: u8 = 0x00;

// Display/cursor shift
const LCD_LEFT: u8 = 0x00;
const LCD_CURSOR_MOVE: u8 = 0x00;
const LCD_RIGHT: u8 = 0x04;
const LCD_DISPLAY_MOVE: u8 = 0x08;

// Function setting
const LCD_4BITMODE: u8 = 0x00;
const LCD_8BITMODE: u8 = 0x10;
const LCD_1LINE: u8 = 0x00;
const LCD_2LINE: u8 = 0x08;
const LCD_5X8DOTS: u8 = 0x00;

#[derive(Debug)]
pub struct LcdDriver {
    chip: Chip,
    rs_line: LineHandle,
    rw_line: Option<LineHandle>,
    enable_line: LineHandle,
    data_lines: Vec<Option<LineHandle>>,
    disp_func: u8,
    disp_mode: u8,
    disp_control: u8,
    num_cols: u8,
    num_rows: u8,
    row_offsets: [u8; 4],
}

impl LcdDriver {
    pub fn new(
        cols: u8,
        rows: u8,
        chip_str: &str,
        four_bit_mode: bool,
        rs: u8,
        rw: u8,
        enable: u8,
        d0: u8,
        d1: u8,
        d2: u8,
        d3: u8,
        d4: u8,
        d5: u8,
        d6: u8,
        d7: u8,
    ) -> Result<Self, gpio_cdev::errors::Error> {
        let mut chip = Chip::new(chip_str)?;
        let rs_line = chip
            .get_line(rs as u32)?
            .request(LineRequestFlags::OUTPUT, 0, "lcd")?;

        let rw_line = match rw {
            255 => None,
            _ => Some(
                chip.get_line(rw as u32)?
                    .request(LineRequestFlags::OUTPUT, 0, "lcd")
                    .unwrap(),
            ),
        };

        let mut none_count = 0;
        let mut data_lines: Vec<Option<LineHandle>> = [d0, d1, d2, d3, d4, d5, d6, d7]
            .iter()
            .map(|line_num| match line_num {
                255 => {
                    none_count += 1;
                    None
                }
                _ => Some(
                    chip.get_line(*line_num as u32)
                        .unwrap()
                        .request(LineRequestFlags::OUTPUT, 0, "lcd")
                        .unwrap(),
                ),
            })
            .filter(|x| x.is_some())
            .collect();
        for _i in 0..none_count {
            data_lines.push(None);
        }

        if none_count != 0 && none_count != 4 {
            return Err(errors::Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Wrong number of unused pins",
            )));
        }

        let mut disp_func = if four_bit_mode {
            LCD_4BITMODE | LCD_1LINE | LCD_5X8DOTS
        } else {
            LCD_8BITMODE | LCD_1LINE | LCD_5X8DOTS
        };

        if cols > 1 {
            disp_func |= LCD_2LINE;
        }
        let row_offsets = [0x00, 0x40, 0x00 + cols, 0x40 + cols];

        let enable_line =
            chip.get_line(enable as u32)?
                .request(LineRequestFlags::OUTPUT, 0, "lcd")?;

        let disp_control = LCD_DISPLAY_ON | LCD_CURSOR_OFF | LCD_BLINK_OFF;
        let disp_mode = LCD_ENTRY_LEFT | LCD_ENTRY_SHIFT_DECREMENT;

        rs_line.set_value(0)?;
        enable_line.set_value(0)?;
        if rw_line.is_some() {
            rw_line.as_ref().unwrap().set_value(0)?;
        }

        let mut lcd_struct = LcdDriver {
            chip,
            rs_line,
            rw_line,
            enable_line,
            data_lines,
            disp_func,
            disp_control,
            disp_mode,
            num_cols: cols,
            num_rows: rows,
            row_offsets,
        };

        if (disp_func & LCD_8BITMODE) == 0 {
            lcd_struct.write4bits(0x03)?;
            sleep(Duration::from_micros(4500));

            lcd_struct.write4bits(0x03)?;
            sleep(Duration::from_micros(4500));

            lcd_struct.write4bits(0x03)?;
            sleep(Duration::from_micros(150));

            lcd_struct.write4bits(0x02)?;
        } else {
            lcd_struct.command(LCD_FUNCTION_SET | disp_func)?;
            sleep(Duration::from_micros(4500));

            lcd_struct.command(LCD_FUNCTION_SET | disp_func)?;
            sleep(Duration::from_micros(150));

            lcd_struct.command(LCD_FUNCTION_SET | disp_func)?;
        }

        lcd_struct.command(LCD_FUNCTION_SET | disp_func)?;
        lcd_struct.display()?;
        lcd_struct.clear()?;

        lcd_struct.command(LCD_ENTRY_MODE_SET | disp_mode)?;

        lcd_struct.create_char(Icon::MAIL.index(), Icon::MAIL.char_data())?;
        lcd_struct.create_char(Icon::BELL.index(), Icon::BELL.char_data())?;
        lcd_struct.create_char(Icon::FILLEDBOX.index(), Icon::FILLEDBOX.char_data())?;
        lcd_struct.create_char(Icon::EMPTYBOX.index(), Icon::EMPTYBOX.char_data())?;
        lcd_struct.create_char(Icon::MAIL.index(), Icon::MAIL.char_data())?;
        lcd_struct.create_char(Icon::BELL.index(), Icon::BELL.char_data())?;
        lcd_struct.create_char(Icon::MUSIC.index(), Icon::MUSIC.char_data())?;
        lcd_struct.create_char(Icon::PLAY.index(), Icon::PLAY.char_data())?;
        lcd_struct.create_char(Icon::PAUSE.index(), Icon::PAUSE.char_data())?;

        Ok(lcd_struct)
    }

    pub fn print(&self, disp_str: &str) -> Result<(), Error> {
        for c in unidecode(disp_str).bytes() {
            self.write(c)?
        }
        Ok(())
    }

    pub fn print_wrapped(&self, disp_str: &str) -> Result<(), Error> {
        self.set_cursor(0, 0)?;
        let mut char_count = 0;
        for c in unidecode(disp_str).bytes() {
            self.write(c as u8)?;
            char_count += 1;
            if char_count == 16 {
                self.set_cursor(1, 0)?;
            }
        }
        Ok(())
    }

    pub fn display(&mut self) -> Result<(), Error> {
        self.disp_control &= LCD_DISPLAY_ON;
        self.command(LCD_DISPLAY_CONTROL | self.disp_control)?;
        Ok(())
    }

    pub fn no_display(&mut self) -> Result<(), Error> {
        self.disp_control &= !LCD_DISPLAY_ON;
        self.command(LCD_DISPLAY_CONTROL | self.disp_control)?;
        Ok(())
    }

    pub fn clear(&self) -> Result<(), Error> {
        self.command(LCD_CLEAR_DISPLAY)?;
        sleep(Duration::from_micros(2000));
        Ok(())
    }

    pub fn home(&self) -> Result<(), Error> {
        self.command(LCD_RETURN_HOME)?;
        sleep(Duration::from_micros(2000));
        Ok(())
    }

    pub fn set_cursor(&self, mut row: u8, col: u8) -> Result<(), Error> {
        let max_lines = self.row_offsets.len() as u8;
        if row >= max_lines {
            row = max_lines - 1;
        }
        if row >= self.num_cols {
            row = self.num_cols - 1;
        }
        self.command(LCD_SET_DDRAM_ADDR | (col + self.row_offsets[row as usize]))
    }

    fn send(&self, val: u8, mode: u8) -> Result<(), errors::Error> {
        self.rs_line.set_value(mode)?;
        if self.rw_line.is_some() {
            self.rw_line.as_ref().unwrap().set_value(0)?;
        }
        if (self.disp_func & LCD_8BITMODE) != 0 {
            self.write8bits(val)?;
        } else {
            self.write4bits(val >> 4)?;
            self.write4bits(val)?;
        }

        Ok(())
    }

    fn pulse_enable(&self) -> Result<(), errors::Error> {
        self.enable_line.set_value(0)?;
        sleep(Duration::from_micros(10));
        self.enable_line.set_value(1)?;
        sleep(Duration::from_micros(10));
        self.enable_line.set_value(0)?;
        sleep(Duration::from_micros(100));
        Ok(())
    }

    fn write4bits(&self, val: u8) -> Result<(), errors::Error> {
        for i in 0..4 {
            self.data_lines[i]
                .as_ref()
                .unwrap()
                .set_value((val >> i as u8) & 0x01)?;
        }
        self.pulse_enable()?;
        Ok(())
    }

    fn write8bits(&self, val: u8) -> Result<(), errors::Error> {
        for i in 0..8 {
            if self.data_lines[i].is_some() {
                self.data_lines[i]
                    .as_ref()
                    .unwrap()
                    .set_value((val >> i as u8) & 0x01)?;
            }
        }
        self.pulse_enable()?;
        Ok(())
    }

    pub fn command(&self, val: u8) -> Result<(), Error> {
        self.send(val, 0)
    }

    pub fn write(&self, val: u8) -> Result<(), Error> {
        self.send(val, 1)
    }

    pub fn create_char(&self, mut loc: u8, charmap: [u8; 8]) -> Result<(), Error> {
        loc &= 0x07; // There are only 8 locations (0-7)
        self.command(LCD_SET_CGRAM_ADDR | (loc << 3))?;
        for i in 0..8 {
            self.write(charmap[i])?
        }
        Ok(())
    }

    pub fn get_rows(&self) -> u8 {
        self.num_rows
    }

    pub fn get_cols(&self) -> u8 {
        self.num_cols
    }
}
