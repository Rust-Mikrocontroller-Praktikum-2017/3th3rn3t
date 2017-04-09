//! # Driver for the stm32f7_discovery rng module
//! Use at your own risk in the following way.
//! ````
//! let mut random = rng::init(rng, rcc);
//! match random.poll_and_get() {
//!
//!         Ok(random_number) => {
//!             println!("Got a random number {}", random_number);
//!         }
//!
//!         Err(_) => {
//!             println!("Something went wrong");
//!         }
//!     }
//!````

use core::ptr;
use core::result::Result;
use core::ops::Drop;
use bit_field::BitField;
use stm32f7::board;

const RCC_BASE_ADDR: u32 = 0x4002_3800;
const RCC_AHB2ENR: u32 = RCC_BASE_ADDR + 0x34;

/// Contains state as well as the Rng Struct from embedded::board.
pub struct Rng<'a> {
    last_number: u32,
    counter: u32,
    board_rng: &'a mut board::rng::Rng

}

///Any of the errors (except AlreadyEnabled) can usually be resolved by initializing this
///struct again.
#[derive(Debug)]
pub enum ErrorType {
    CECS,
    SECS,
    CEIS,
    SEIS,
    AlreadyEnabled,
    NotReady
}



impl<'a> Rng<'a> {

    pub fn init(rng: &'a mut board::rng::Rng, rcc: &mut board::rcc::Rcc) -> Result<Rng<'a>, ErrorType> {

        let control_register = rng.cr.read().rngen();
        // let reg_content = unsafe { ptr::read_volatile(RNG_CR as *mut u32) };
        if control_register {
            return Err(ErrorType::AlreadyEnabled);
        }

        let mut rng = Rng { last_number: 0x0, counter: 0x0, board_rng: rng };
        rcc.ahb2enr.update(|r| r.set_rngen(true));

        rng.board_rng.cr.update(|r| {
            r.set_ie(false);
            r.set_rngen(true);
        });

        Ok(rng)
    }


    /// For Testing purposes. Do not use except for debugging!
    pub fn tick(&mut self) -> u32 {
        match self.poll_and_get() {

            Ok(number) => {
                return number;
            }
            Err(e) => {
                match e {
                    _ => {
                        return 0;
                    }
                }
            }
        }
    }


    /// Actually try to acquire some random number
    pub fn poll_and_get(&mut self) -> Result<u32, ErrorType> {

        // let status = unsafe { ptr::read_volatile(RNG_STATUS as *mut u32) };
        let status = self.board_rng.sr.read();

        if status.ceis() {
            self.reset();
            return Err(ErrorType::CEIS);
        }
        if status.seis() {
            self.reset();
            return Err(ErrorType::SEIS);
        }

        if status.cecs() {
            return Err(ErrorType::CECS);
        }
        if status.secs() {
            self.reset();
            return Err(ErrorType::SECS);
        }
        if status.drdy() {
            let data = self.board_rng.dr.read().rndata();
            if data != self.last_number {
                self.last_number = data;
                self.counter = 0;
                return Ok(data);
            }
        }
        self.counter += 1;
        if self.counter > 80 {
            self.reset();
            self.counter = 0;
        }
        // data was not ready, try again!
        Err(ErrorType::NotReady)
    }

    pub fn reset(&mut self) {
        self.board_rng.cr.update(|r| r.set_rngen(false));
        self.board_rng.cr.update(|r| r.set_ie(false));
        self.board_rng.cr.update(|r| r.set_rngen(true));
    }


    fn disable_cr(&mut self, rcc: &mut board::rcc::Rcc) {

        self.board_rng.cr.update(|r| r.set_rngen(false));
        self.board_rng.cr.update(|r| r.set_ie(false));
        rcc.ahb2enr.update(|r| r.set_rngen(false));
    }


}
impl<'a> Drop for Rng<'a> {

    fn drop(&mut self) {
        unsafe {
            let mut reg = ptr::read_volatile(RCC_AHB2ENR as *const u32);
            reg.set_bit(6, false);
            ptr::write_volatile(RCC_AHB2ENR as *mut u32, reg);
        }
        self.board_rng.cr.update(|r| r.set_rngen(false));
    }
}
