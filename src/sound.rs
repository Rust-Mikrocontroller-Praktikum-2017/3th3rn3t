use stm32f7::board::sai::{Sai,Bclrfr};
use stm32f7::board::rcc::Rcc;
use stm32f7::board::sai;
use embedded::interfaces::gpio::Gpio;
use i2c;
use bit_field::BitField;
use stm32f7::system_clock;

const WM8994_ADDRESS: i2c::Address = i2c::Address::bits_7(0b0011010);

pub struct Sound {}

impl Sound {

    pub fn init(sai: &mut Sai, i2c_3: &mut i2c::I2C, rcc: &mut Rcc, gpio: &mut Gpio) -> Self {


        sai.acr1.update(|r| r.set_saiaen(false));
        sai.bcr1.update(|r| r.set_saiben(false));

        println!("before wait for disable of saien");
        while sai.acr1.read().saiaen() {}
        while sai.bcr1.read().saiben() {}
        println!("after wait for disable of saien");

        sai.bim.write(Default::default());

        // clear like all flags
        {
            let mut clear_all_flags = Bclrfr::default();
            clear_all_flags.set_lfsdet(true); // Clear late frame synchronization detection flag
            clear_all_flags.set_cafsdet(true); // Clear anticipated frame synchronization detection flag
            clear_all_flags.set_cnrdy(true); // Clear codec not ready flag
            clear_all_flags.set_wckcfg(true); // Clear wrong clock configuration flag
            clear_all_flags.set_mutedet(true); // Clear mute detection flag
            clear_all_flags.set_ovrudr(true); // Clear overrun / underrun
            sai.bclrfr.write(clear_all_flags);
        }

        // Flush the fifo
        sai.bcr2.update(|r| r.set_fflus(true)); // fifo_flush

        // Disable the PLLI2S
        rcc.cr.update(|r| r.set_plli2son(false));
        println!("before wait for pllclock");
        while rcc.cr.read().plli2srdy() {}
        println!("after wait for pllclock");

        //TODO set clock dividers here
        // SAI_CLK_x = SAI_CLK(first level)/PLLI2SDIVQ
        rcc.dkcfgr1.update(|r| r.set_plli2sdiv(1 - 1));


        rcc.dkcfgr1.update(|r| r.set_sai2sel(0)); // sai2_clock_source pllsaiq/pllsaiq

        // SET PLLI2S_CLK 344 MHZ, PLLI2DIVQ = 7
        rcc.plli2scfgr
            .update(|r| {
                r.set_plli2sn(344);
                r.set_plli2sq(7);
            });

        // SAI_CLK_x = SAI_CLK(first level)/PLLI2SDIVQ
        rcc.dkcfgr1.update(|r| r.set_plli2sdiv(1 - 1));


        // Enable the PLLI2S
        rcc.cr.update(|r| r.set_plli2son(true));
        println!("before wait for pllclock VER 2");
        while !rcc.cr.read().plli2srdy() {}
        println!("after wait for pllclock VER 2");

        rcc.apb2enr.update(|r| r.set_sai2en(true));

        sai.gcr.update(|r| {
            r.set_syncout(0b01);
        });

        sai.acr1.update(|r| {
            // clock must be present
            r.set_saiaen(true);
            r.set_mode(0b00); // PROBABLY this if not 0b01
            r.set_mono(false);
            r.set_ds(0b100);
            r.set_prtcfg(0b00);
            r.set_mono(false);
            r.set_mcjdiv(0b0010);
            r.set_nodiv(false);
            r.set_out_dri(false);
        });

        // configure frame
        {
            let mut afrcr = sai::Afrcr::default();
            afrcr.set_frl(64 - 1); // frame_length
            afrcr.set_fsall(32 - 1); // sync_active_level_length
            afrcr.set_fsdef(true); // frame_sync_definition
            afrcr.set_fspol(false); // frame_sync_polarity
            afrcr.set_fsoff(true); // frame_sync_offset
            sai.afrcr.write(afrcr);
        }


        // configure slots
        {
            let mut aslotr = sai::Aslotr::default();
            let mut slots = 0x0u16;
            slots.set_bit(1, true);
            slots.set_bit(3, true);
            aslotr.set_sloten(slots);
            aslotr.set_fboff(0);
            aslotr.set_slotsz(0b00);
            sai.aslotr.write(aslotr);
        }


        sai.acr2.update(|r| {
            r.set_mute(false);
        });

        sai.aslotr.update(|r| {
            r.set_sloten(0xff);
        });

        // read status bits
        {
            let reg = sai.asr.read();
            if reg.wckcfg() {
                println!("Configured clock is wrong!");
            }
            if reg.ovrudr() {
                println!("Fifo Overrun/Underrun detected");
            }

            println!("fifo threshhold is {}, should be 0 at this point", reg.flvl());

        }

        if sai.acr1.read().dmaen() {
            println!("DMA is enabled");
        } else {
            println!("DMA is disabled");
        }

        // match sai.acr1.read().prtcfg() {
        //     0b00 => println!("free form mode"),
        //     0b01 => println!("spdif form mode"),
        //     0b10 => println!("ac97 form mode"),
        //      _    => println!("reserved mode bits set")
        // }

        println!("before wait for saienable");
        while !sai.acr1.read().saiaen() {}
        println!("after wait for saienable");

        self::config_gpio(gpio);

        let ret = i2c_3.connect::<u16, _>(WM8994_ADDRESS, |mut conn| {

            // read and check device family ID
            assert_eq!(conn.read(0).ok(), Some(0x8994));
            // reset device
            try!(conn.write(0, 0));


            // wm8994 Errata Work-Arounds
            try!(conn.write(0x102, 0x0003));
            try!(conn.write(0x817, 0x0000));
            try!(conn.write(0x102, 0x0000));

            // Enable VMID soft start (fast), Start-up Bias Current Enabled
            try!(conn.write(0x39, 0x006C));

            // Enable bias generator, Enable VMID
            try!(conn.write(0x01, 0x0003));

            system_clock::wait(50);

            // AIF1DAC1X
            {
                let mut bits = conn.read(0x0005)?;
                bits.set_bit(9, true);
                bits.set_bit(8, true);
                conn.write(0x0005, bits);
            }

            // AIF1DACXL_TO_DAC1L
            {
                let mut bits = 0x0;
                bits.set_bit(0, true);
                bits.set_bit(1, true);
                conn.write(0x0601, bits);
            }


            // AIF1DACXR_TO_DAC1R
            {
                let mut bits = 0x0;
                bits.set_bit(0, true);
                bits.set_bit(1, true);
                conn.write(0x0602, bits);
            }

            // DAC1X VOL SET
            {
                let mut bits = 0xC0;
                bits.set_bit(9, false); // UN-MUTE
                bits.set_bit(8, true); // Update L AND R simultaneously
                // bits.set_range(0..8, 0xC0); // 0db WHY does this fail? TODO
                conn.write(0x0610, bits);
            }


            // DAC1L to HPOUT1L
            {
                let mut bits = conn.read(0x002d)?;
                bits.set_bit(8, true); // DAC1L
                conn.write(0x002d, bits);
            }


            // DAC1R to HPOUT1R
            {
                let mut bits = conn.read(0x002e)?;
                bits.set_bit(8, true); // DAC1R
                conn.write(0x002e, bits);
            }

            // AIF1CLK EN, technically unnecessary but for reference
            {
                let mut bits = 0x0;
                bits.set_bit(0, true); // clock enable
                conn.write(0x0200, bits);
            }


            // initiate headphone cold startup sequence
            conn.write(0x0110, 0x8100);

            Ok(())
        });


        Sound {}
    }

    pub fn tick(&mut self) {
        println!("Sound Tick");
    }



    pub fn put_data(&mut self, sai: &mut Sai, i2c_3: &mut i2c::I2C, data: u32) {

        // let ret = i2c_3.connect::<u16, _>(WM8994_ADDRESS, |mut conn| {

        //     let mut reg = conn.read(0x1)?;

        //     reg.set_bit(11, false);
        //     reg.set_bit(9, false);
        //     reg.set_bit(8, false);

        //     conn.write(0x1, reg)?;

        //     reg.set_bit(11, true);
        //     reg.set_bit(9, true);
        //     reg.set_bit(8, true);

        //     conn.write(0x1, reg)?;

        //     Ok(())
        // });

        // match ret {
        //     Err(e) => println!("Error connect during put_data"),
        //     _ => {}
        // };

        if sai.asr.read().flvl() != 0b101 {
            println!("setting data");
            sai.adr.update(|reg| reg.set_data(data));
        }

        system_clock::wait(50);
    }
}

fn config_gpio(gpio: &mut Gpio) {
        use embedded::interfaces::gpio::{OutputType, OutputSpeed, AlternateFunction, Resistor};
        use embedded::interfaces::gpio::Port::*;
        use embedded::interfaces::gpio::Pin::*;

        // block A (master)
        let sai2_fs_a = (PortI, Pin7);
        let sai2_sck_a = (PortI, Pin5);
        let sai2_sd_a = (PortI, Pin6);
        let sai2_mclk_a = (PortI, Pin4);
        // block B (synchronous slave)
        let sai2_sd_b = (PortG, Pin10);

        let pins = [sai2_fs_a, sai2_sck_a, sai2_sd_a, sai2_mclk_a, sai2_sd_b];
        gpio.to_alternate_function_all(&pins,
                                       AlternateFunction::AF10,
                                       OutputType::PushPull,
                                       OutputSpeed::High,
                                       Resistor::NoPull)
            .unwrap();
    }
