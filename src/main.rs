#![no_std]
#![no_main]

#![feature(asm)]
#![feature(const_fn)]

extern crate stm32f7_discovery as stm32f7;
extern crate r0;
extern crate bit_field;

use stm32f7::{audio, ethernet, sdram, system_clock, board, embedded, touch, i2c, lcd};

#[macro_use]
mod semi_hosting;
mod font;
mod graphics;
mod rng;

use rng::{Rng,ErrorType};

#[no_mangle]
pub unsafe extern "C" fn reset() -> ! {

    extern "C" {
        static __DATA_LOAD: u32;
        static __DATA_END: u32;
        static mut __DATA_START: u32;

        static mut __BSS_START: u32;
        static mut __BSS_END: u32;
    }

    let data_load = &__DATA_LOAD;
    let data_start = &mut __DATA_START;
    let data_end = &__DATA_END;

    let bss_start = &mut __BSS_START;
    let bss_end = &__BSS_END;

    // initializes the .data section (copy the data segment initializers from flash to RAM)
    r0::init_data(data_start, data_end, data_load);
    // zeroes the .bss section
    r0::zero_bss(bss_start, bss_end);

    stm32f7::heap::init();

    unsafe {
        let scb = stm32f7::cortex_m::peripheral::scb_mut();
        scb.cpacr.modify(|v| v | 0b1111 << 20);
    }

    main(board::hw());
}


fn main(hw: board::Hardware) -> ! {

    use embedded::interfaces::gpio::{self,Gpio};


    let board::Hardware {
        rcc,
        pwr,
        flash,
        fmc,
        ltdc,
        gpio_a,
        gpio_b,
        gpio_c,
        gpio_d,
        gpio_e,
        gpio_f,
        gpio_g,
        gpio_h,
        gpio_i,
        gpio_j,
        gpio_k,
        i2c_3,
        sai_2,
        syscfg,
        ethernet_mac,
        ethernet_dma,
        ..
    } = hw;

    let mut gpio = Gpio::new(gpio_a,
                             gpio_b,
                             gpio_c,
                             gpio_d,
                             gpio_e,
                             gpio_f,
                             gpio_g,
                             gpio_h,
                             gpio_i,
                             gpio_j,
                             gpio_k);

    system_clock::init(rcc, pwr, flash);
    rcc.ahb1enr.update(|r| {

        r.set_gpioaen(true);
        r.set_gpioben(true);
        r.set_gpiocen(true);
        r.set_gpioden(true);
        r.set_gpioeen(true);
        r.set_gpiofen(true);
        r.set_gpiogen(true);
        r.set_gpiohen(true);
        r.set_gpioien(true);
        r.set_gpiojen(true);
        r.set_gpioken(true);
    });

    let led_pin = (gpio::Port::PortI, gpio::Pin::Pin1);

    sdram::init(rcc, fmc, &mut gpio);

    let mut lcd = lcd::init(ltdc, rcc, &mut gpio);

    i2c::init_pins_and_clocks(rcc, &mut gpio);
    let mut i2c_3 = i2c::init(i2c_3);

    audio::init_sai_2_pins(&mut gpio);
    audio::init_sai_2(sai_2, rcc);
    assert!(audio::init_wm8994(&mut i2c_3).is_ok());

    let mut led = gpio.to_output(led_pin,
                                 gpio::OutputType::PushPull,
                                 gpio::OutputSpeed::Low,
                                 gpio::Resistor::NoPull,)
        .expect("led pin already in use");


    // let mut eth_device = ethernet::EthernetDevice::new(Default::default(),
    //                                                    Default::default(),
    //                                                    rcc,
    //                                                    syscfg,
    //                                                    &mut gpio,
    //                                                    ethernet_mac,
                                                       // ethernet_dma);
    // if let Err(e) = eth_device {
    //     println!("ethernet init failed: {:?}", e);
    // }

    let mut rng = rng::enable().expect("rng already enabled");

    let mut last_toggle_ticks = system_clock::ticks();

    lcd.clear_screen();
    touch::check_family_id(&mut i2c_3).unwrap();

    loop {

        let ticks = system_clock::ticks();
        if (ticks - last_toggle_ticks)  > 1500 {
            let current_led_state = led.get();
            led.set(!current_led_state);
            last_toggle_ticks = ticks;
        }

    }
}
