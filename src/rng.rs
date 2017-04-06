use core::ptr;
use core::result::Result;
use bit_field::BitField;

const RNG_BASE_ADDR: u32 = 0x5006_0800;
const RNG_CR: u32 = RNG_BASE_ADDR + 0x0;
const RNG_STATUS: u32 = RNG_BASE_ADDR + 0x4;
const RNG_DATA: u32 = RNG_BASE_ADDR + 0x8;

pub struct Rng(u32, u32);

#[derive(Debug)]
pub enum ErrorType {
    CECS,
    SECS,
    AlreadyEnabled,
    NotReady
}

fn disable_cr () {

    let mut bits = unsafe { ptr::read_volatile(RNG_CR as *mut u32) };
    bits.set_bit(2, false);
    bits.set_bit(3, false);

    unsafe { ptr::write_volatile(RNG_CR as *mut u32, bits) };
}

fn enable_cr () {

    let mut bits = 0;
    bits.set_bit(2, true);

    unsafe { ptr::write_volatile(RNG_CR as *mut u32, bits) };
}

pub fn enable() -> Result<Rng, ErrorType> {

    let reg_content = unsafe { ptr::read_volatile(RNG_CR as *mut u32) };
    if reg_content.get_bit(2) {
        return Err(ErrorType::AlreadyEnabled);
    }

    let rng = Rng(0x0, 0x0);

    enable_cr();

    Ok(rng)
}

impl Rng {
    pub fn poll_and_get(&mut self) -> Result<u32, ErrorType> {

        let status = unsafe { ptr::read_volatile(RNG_STATUS as *mut u32) };

        if !status.get_bit(1) {
            if !status.get_bit(2) {
                if status.get_bit(0) {
                    let data = unsafe { ptr::read_volatile(RNG_DATA as *mut u32) };
                    if data != self.0 {
                        self.0 = data;
                        self.1 = 0;
                        return Ok(data);
                    }
                }
            } else {
                disable_cr();
                enable_cr(); // recommended by manual
                return Err(ErrorType::SECS);
            }
        } else {
            return Err(ErrorType::CECS);
        }

        self.1 += 1;
        if self.1 > 40 {
            println!("RNG reset");
            disable_cr();
            enable_cr();
            self.1 = 0;
        }
        // data was not ready, try again!
        Err(ErrorType::NotReady)
    }

    pub fn disable(self) {
        unsafe { ptr::write_volatile(RNG_CR as *mut u32, 0x0) };
    }
}


pub fn tick(rng: &mut Rng) {
    match rng.poll_and_get() {

        Ok(number) => {
            println!("Random number received {}", number);
        }
        Err(e) => {
            match e {
                ErrorType::CECS => {
                    println!("CECS error received");
                }
                ErrorType::SECS => {
                    println!("SECS error received");
                }
                ErrorType::AlreadyEnabled => {
                    unreachable!();
                }
                ErrorType::NotReady => {
                    println!("No Random Number Ready Yet");
                }
            }
        }
    }
}


