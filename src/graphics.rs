use stm32f7::lcd::{Lcd,Color};
use stm32f7::system_clock;
use core::ptr;

#[feature(inclusive_range_syntax)]

pub fn draw_square(x: u16, y: u16, len: u16, color: u16, lcd: &mut Lcd) {

    let addr: u32 = 0xC000_0000 + ( 480 * 272 * 2);

    // let x = x as u32;
    // let y = y as u32;
    // let len = len as u32;

    // for i in x..(len+x) {
    //     for j in y..(len+y) {
    //         let mut pixel = j * 480 + i;
    //         let mut pixel_pos = (addr + pixel * 2) as *mut u16;
    //         unsafe { ptr::write_volatile(pixel_pos, color ) };
    //     }
    // }
    for i in x..(x+len) {
//        let mut pixel = y * 480 + i;
//        let mut pixel_pos = (addr + pixel * 2) as *mut u16;
//        unsafe { ptr::write_volatile(pixel_pos, color ) };
//
//        pixel =  y * 480 + i + len - 1;
//        let mut pixel_pos = (addr + pixel * 2) as *mut u16;
//        unsafe { ptr::write_volatile(pixel_pos, color ) };
//
        lcd.print_point_color_at(i, y, color);
        // system_clock::wait(1);
        lcd.print_point_color_at(i, y + len - 1, color);
    }


    // let pixel = i * 480 + j;
    // let pixel_color = (addr + pixel * 2) as *mut u16;
    // unsafe { ptr::write_volatile(pixel_color, colors[(i / 10) as usize & 7]) };

    // let pixel = i * 480 + j;
    // let pixel_color = (addr + pixel * 2) as *mut u16;
    // unsafe { ptr::write_volatile(pixel_color, colors[(i / 10) as usize & 7]) };


    for i in y..(y+len) {
            lcd.print_point_color_at(x, i, color);
        //
        //     // system_clock::wait(1);
            lcd.print_point_color_at(x + len - 1, i, color);
    }

}

