use stm32f7::lcd::{self, Lcd, Color};
use stm32f7::{self, system_clock, touch};
use i2c::{self, I2C};
use core::ptr;
use collections::{vec, Vec};
use board::rcc::Rcc;
use board::ltdc::Ltdc;
use embedded::interfaces::gpio::Gpio;
use alloc::rc::{self, Rc};
use collections::boxed::{self, Box};

enum Button {
    ColorSquareButton {
        x: u16,
        y: u16,
        len: u16,
        color: u16
    }
}
/*
 *
 *trait Button {
 *    fn touched_inside(&self, x: u16, y: u16) -> bool;
 *    fn draw(&self, lcd: &mut Lcd);
 *}
 *
 */
impl Button {

    pub fn touched_inside(&self, touch_x: u16, touch_y: u16) -> bool {
        match self {
            &Button::ColorSquareButton {x, y, len, color} => (x <= touch_x && touch_x <= (x + len) && y <= touch_y && touch_y <= (y + len)),
        }
    }

    pub fn draw(&self, lcd: &mut Lcd) {
        match self {
            &Button::ColorSquareButton {x, y, len, color} => Graphics::draw_square_filled(lcd, x, y, len, color),
        }
    }

}

pub struct Graphics {
    lcd: Lcd,
    buttons: Vec<Rc<Button>>,
    touch_color: u16
}

impl Graphics {
    // TODO: this mut before gpio is strange
    pub fn init(ltdc: &'static mut Ltdc, rcc: &mut Rcc, mut gpio: &mut Gpio, i2c_3: &mut I2C) -> Self {
        let mut graphics = Graphics {
            lcd: lcd::init(ltdc, rcc, &mut gpio),
            buttons: Vec::new(),
            touch_color: 0xffff
        };
        touch::check_family_id(i2c_3).unwrap();
        graphics
    }

    pub fn prepare(&mut self) {
        self.lcd.clear_screen();
        self.lcd.set_background_color(Color::from_hex(0x0));

        self.touch_color = 0xffff;

        let b1 = Rc::new(Button::ColorSquareButton {x: 10, y: 10,  len: 50, color: self.touch_color});
        let b2 = Rc::new(Button::ColorSquareButton {x: 10, y: 70,  len: 50, color: 0xff00});
        let b3 = Rc::new(Button::ColorSquareButton {x: 10, y: 130, len: 50, color: 0xaacc});
        let b4 = Rc::new(Button::ColorSquareButton {x: 10, y: 190, len: 50, color: 0xccaa});

        self.buttons.push(b1.clone());
        self.buttons.push(b2.clone());
        self.buttons.push(b3.clone());
        self.buttons.push(b4.clone());

        for button in self.buttons.iter() {
            button.draw(&mut self.lcd);
        }

    }

    pub fn tick(&mut self, i2c_3: &mut I2C) {

        for touch in &touch::touches(i2c_3).unwrap() {
            let mut color_changed = false;

            // check if one of the color buttons was touched
            for button in self.buttons.iter() {
                match **button {
                    Button::ColorSquareButton {x, y, len, color} => {
                        if !color_changed && button.touched_inside(touch.x, touch.y) {
                            self.touch_color = color;
                            color_changed = true;
                        }
                    }
                }
            }

            // draw a point if this touch didn't touch a color button
            if !color_changed {
                self.lcd.print_point_color_at(touch.x, touch.y, self.touch_color);
            }
        }
    }

    pub fn draw_square(&mut self, x: u16, y: u16, len: u16, color: u16) {

        for i in x..(x+len) {
            self.lcd.print_point_color_at(i, y, color);
            self.lcd.print_point_color_at(i, y + len - 1, color);
        }

        for i in y..(y+len) {
            self.lcd.print_point_color_at(x, i, color);
            self.lcd.print_point_color_at(x + len - 1, i, color);
        }
    }

    pub fn draw_square_filled(lcd: &mut Lcd, x: u16, y: u16, len: u16, color: u16) {
        for i in x..(x+len) {
            for j in y..(y+len) {
                lcd.print_point_color_at(i, j, color);
            }
        }
    }
}
