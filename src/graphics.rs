use stm32f7::lcd::{self, Lcd, Color};
use stm32f7::{self, system_clock, touch};
use i2c::{self, I2C};
use core::ptr;
use collections::{vec,Vec};
use board::rcc::Rcc;
use board::ltdc::Ltdc;
use embedded::interfaces::gpio::{Gpio};

#[feature(inclusive_range_syntax)]

pub struct ColorSquare {
    x: u16,
    y: u16,
    len: u16,
    color: u16
}

impl ColorSquare {
    pub const fn new(x: u16, y: u16, len: u16, color: u16) -> Self {
        ColorSquare{x, y, len, color}
    }

    pub fn touched_inside(&self, x: u16, y: u16) -> bool {
        self.x <= x && x <= (self.x + self.len) && self.y <= y && y <= (self.y + self.len)
    }

    pub fn draw(&self, graphics: &Graphics) {
        graphics.draw_square_filled(self.x, self.y, self.len, self.color);
    }

    pub fn get_color(&self) -> u16 {
        self.color
    }
}

// TODO: ohne mut, oder unsafe Cell (interior mutability)
pub struct Graphics {
    lcd: Lcd,
    i2c_3: I2C,
    color_buttons: Vec<ColorSquare>,
    touch_color: u16
}

impl Graphics {
    // TODO: this explicit indication of i2c's type is strange
    pub fn init(ltdc: &'static mut Ltdc, rcc: &mut Rcc, gpio: &mut Gpio, i2c: &'static mut stm32f7::board::i2c::I2c) -> Self {
        i2c::init_pins_and_clocks(rcc, &mut gpio);
        let mut graphics = Graphics {
            lcd: lcd::init(ltdc, rcc, &mut gpio),
            i2c_3: i2c::init(i2c),
            color_buttons: Vec::new(),
            touch_color: 0xffff
        };
        touch::check_family_id(&mut graphics.i2c_3).unwrap();
        graphics
    }

    pub fn prepare(&self) {
        self.color_buttons = vec![
            ColorSquare::new(10, 10, 50, 0xffff),
            ColorSquare::new(10, 70, 50, 0xff00),
            ColorSquare::new(10, 130, 50, 0xaacc),
            ColorSquare::new(10, 190, 50, 0xccaa)];

        for color_button in self.color_buttons.iter() {
            color_button.draw(&self);
        }

        self.touch_color = self.color_buttons[0].get_color();
    }

    pub fn tick(&self) {

        for touch in &touch::touches(&mut self.i2c_3).unwrap() {
            let mut color_changed = false;

            // check if one of the color buttons was touched
            for color_button in self.color_buttons.iter() {
                if !color_changed && color_button.touched_inside(touch.x, touch.y) {
                    self.touch_color = color_button.get_color();
                    color_changed = true;
                }
            }

            // draw a point if this touch didn't touch a color button
            if !color_changed {
                self.lcd.print_point_color_at(touch.x, touch.y, self.touch_color);
            }
        }
    }

    pub fn draw_square(&self, x: u16, y: u16, len: u16, color: u16) {

        for i in x..(x+len) {
            self.lcd.print_point_color_at(i, y, color);
            self.lcd.print_point_color_at(i, y + len - 1, color);
        }

        for i in y..(y+len) {
            self.lcd.print_point_color_at(x, i, color);
            self.lcd.print_point_color_at(x + len - 1, i, color);
        }
    }

    pub fn draw_square_filled(&self, x: u16, y: u16, len: u16, color: u16) {
        for i in x..(x+len) {
            for j in y..(y+len) {
                self.lcd.print_point_color_at(i, j, color);
            }
        }
    }
}
