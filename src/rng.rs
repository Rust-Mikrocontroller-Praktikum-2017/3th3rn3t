use core::ptr;
use core::result::Result;
use bit_field::BitField;
use ::system_clock;

const RNG_BASE_ADDR: u32 = 0x5006_0800;
const RNG_CR: u32 = RNG_BASE_ADDR + 0x0;
const RNG_STATUS: u32 = RNG_BASE_ADDR + 0x4;
const RNG_DATA: u32 = RNG_BASE_ADDR + 0x8;

const RCC_BASE_ADDR: u32 = 0x4002_3800;
const RCC_AHB2RSTR: u32 = RCC_BASE_ADDR + 0x14;
const RCC_AHB2ENR: u32 = RCC_BASE_ADDR + 0x34;

pub struct Rng(u32, u32);

#[derive(Debug)]
pub enum ErrorType {
    CECS,
    SECS,
    CEIS,
    SEIS,
    AlreadyEnabled,
    NotReady
}


pub fn enable() -> Result<Rng, ErrorType> {

    let reg_content = unsafe { ptr::read_volatile(RNG_CR as *mut u32) };
    if reg_content.get_bit(2) {
        return Err(ErrorType::AlreadyEnabled);
    }

    enable_cr();
    let rng = Rng(0x0, 0x0);
    Ok(rng)
}


fn enable_cr () {

    let mut bits_rcc_en = 0;
    bits_rcc_en.set_bit(6, true);

    let mut bits_rng_cr = 0;
    bits_rng_cr.set_bit(2, true);

    let mut bits_rcc_ahb2rstr = 0;
    bits_rcc_ahb2rstr.set_bit(6, true);

    unsafe {
        // reset
        // ptr::write_volatile(RCC_AHB2RSTR as *mut u32, bits_rcc_ahb2rstr);
        // assert_eq!(ptr::read_volatile(RCC_AHB2RSTR as *mut u32), bits_rcc_ahb2rstr);

        // clock enable
        ptr::write_volatile(RCC_AHB2ENR as *mut u32, bits_rcc_en);
        // assert_eq!(ptr::read_volatile(RCC_AHB2ENR as *mut u32), bits_rcc_en);

        // device enable
        ptr::write_volatile(RNG_CR as *mut u32, bits_rng_cr);
        // assert_eq!(ptr::read_volatile(RNG_CR as *mut u32), bits_rng_cr);

        let test = ptr::read_volatile(RNG_CR as *mut u32);
        if !test.get_bit(2) {
            println!("Rng disabled again");
        }
    }
}


fn disable_cr () {

    let mut bits = unsafe { ptr::read_volatile(RNG_CR as *mut u32) };
    bits.set_bit(2, false);
    bits.set_bit(3, false);

    unsafe {
        ptr::write_volatile(RNG_CR as *mut u32, bits);
        assert_eq!(ptr::read_volatile(RNG_CR as *mut u32), bits);
    };
}


impl Rng {
    pub fn poll_and_get(&mut self) -> Result<u32, ErrorType> {

        let status = unsafe { ptr::read_volatile(RNG_STATUS as *mut u32) };

        if status.get_bit(5) {
            disable_cr();
            enable_cr();
            return Err(ErrorType::CEIS);
        }
        if status.get_bit(6) {
            disable_cr();
            enable_cr();
            return Err(ErrorType::SEIS);
        }
        // println!("Resetting clock, CEIS or SEIS was set");

        if status.get_bit(1) {
            return Err(ErrorType::CECS);
        }
        if status.get_bit(2) {
            disable_cr();
            enable_cr(); // recommended by manual
            return Err(ErrorType::SECS);
        }
        if status.get_bit(0) {
            let data = unsafe { ptr::read_volatile(RNG_DATA as *mut u32) };
            if data != self.0 {
                self.0 = data;
                self.1 = 0;
                return Ok(data);
            }
        }
        self.1 += 1;
        if self.1 > 80 {
            // println!("RNG reset");
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
                _ => {
                    println!("Error: {:?}", e);
                }
            }
        }
    }
}


