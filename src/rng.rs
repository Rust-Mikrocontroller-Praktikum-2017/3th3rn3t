use core::ptr;
use core::result::Result;
use bit_field::BitField;

const rng_base_addr: u32 = 0x5006_0800;
const rng_cr: u32 = rng_base_addr + 0x0;
const rng_status: u32 = rng_base_addr + 0x4;
const rng_data: u32 = rng_base_addr + 0x8;

pub struct Rng(u32);

#[derive(Debug)]
pub enum ErrorType {
    CEIS,
    SEIS,
    AlreadyEnabled
}


fn enable_cr () {


    // clear SEIS bit

    let mut data = unsafe { ptr::read_volatile(rng_status as *mut u32) };
    let mut d = data.clone();
    d.set_bit(6, false);

    unsafe { ptr::write_volatile(rng_status as *mut u32, d)};

    let mut bits = unsafe { ptr::read_volatile(rng_cr as *mut u32) };
    bits.set_bit(2, true);
    bits.set_bit(3, false);

    unsafe { ptr::write_volatile(rng_cr as *mut u32, bits) };
}


pub fn enable() -> Result<Rng, ErrorType> {

    let reg_content = unsafe { ptr::read_volatile(rng_cr as *mut u32) };
    if reg_content.get_bit(2) {
        return Err(ErrorType::AlreadyEnabled);
    }

    let rng = Rng(0x0);

    enable_cr();

    Ok(rng)
}

impl Rng {
    pub fn poll_and_get(&mut self) -> Result<Option<u32>, ErrorType> {

        let content = unsafe { ptr::read_volatile(rng_status as *mut u32) };

        if !content.get_bit(5) {
            if !content.get_bit(6) {
                if content.get_bit(0) {
                    let data = unsafe { ptr::read_volatile(rng_data as *mut u32) };
                    if data != self.0 {
                        self.0 = data;
                        return Ok(Some(data));
                    }
                }
            } else {
                enable_cr(); // recommended by manual
                return Err(ErrorType::SEIS);
            }
        } else {
            return Err(ErrorType::CEIS);
        }

        // data was not ready, try again!
        Ok(None)
    }

    pub fn disable(self) {
        unsafe { ptr::write_volatile(rng_cr as *mut u32, 0x0) };
    }
}
