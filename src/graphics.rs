use stm32f7::lcd::{Lcd,Color};
use stm32f7::system_clock;
use core::ptr;

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

    pub fn draw(&self, lcd: &mut Lcd) {
        draw_square_filled(self.x, self.y, self.len, self.color, lcd);
    }

    pub fn get_color(&self) -> u16 {
        self.color
    }
}


pub fn draw_square(x: u16, y: u16, len: u16, color: u16, lcd: &mut Lcd) {

    for i in x..(x+len) {
        lcd.print_point_color_at(i, y, color);
        lcd.print_point_color_at(i, y + len - 1, color);
    }

    for i in y..(y+len) {
            lcd.print_point_color_at(x, i, color);
            lcd.print_point_color_at(x + len - 1, i, color);
    }
}

pub fn draw_square_filled(x: u16, y: u16, len: u16, color: u16, lcd: &mut Lcd) {
    for i in x..(x+len) {
        for j in y..(y+len) {
            lcd.print_point_color_at(i, j, color);
        }
    }
}