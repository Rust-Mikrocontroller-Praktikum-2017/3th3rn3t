use stm32f7::board::sai::Sai;
use stm32f7::board::rcc::Rcc;
use i2c;
use bit_field::BitField;
use stm32f7::system_clock;

const WM8994_ADDRESS: i2c::Address = i2c::Address::bits_7(0b0011010);

pub struct Sound {}

impl Sound {

    pub fn init(sai: &mut Sai, i2c_3: &mut i2c::I2C, rcc: &mut Rcc) -> Self {

        sai.gcr.update(|r| {
            r.set_syncout(0b01);
        });

        sai.acr1.update(|r| {
            // clock must be present
            r.set_saiaen(false);
            r.set_mode(0b00); // PROBABLY this if not 0b01
            r.set_ds(0b100);
            r.set_prtcfg(0b00);
            r.set_mono(false);
            r.set_mcjdiv(0b0010);
            r.set_out_dri(false);
        });

        sai.acr2.update(|r| {
            r.set_mute(false);
        });

        sai.aslotr.update(|r| {
            r.set_sloten(0xff);
        });
        if sai.asr.read().wckcfg() {
            println!("Configured clock is wrong!");
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

        sai.acr1.update(|r| {
            r.set_saiaen(true);
        });



        let ret = i2c_3.connect::<u16, _>(WM8994_ADDRESS, |mut conn| {

            // let mut hpout_ena = conn.read(0x1)?;
            // let mut hpout_dly = conn.read(0x60)?;

            // hpout_ena.set_bit(8, true);
            // hpout_ena.set_bit(9, true);


            // hpout_dly.set_bit(5, true);
            // hpout_dly.set_bit(1, true);


            // // DO NOT MOVE! OUTP and RMV_SHORT are written in the same register!
            // let mut hpout_outp_rmv_short = hpout_dly;

            // hpout_outp_rmv_short.set_bit(6, true);
            // hpout_outp_rmv_short.set_bit(2, true);
            // hpout_outp_rmv_short.set_bit(7, true);
            // hpout_outp_rmv_short.set_bit(3, true);

            // conn.write(0x1, hpout_ena)?;
            // // forced wait
            // system_clock::wait(1);
            // // DC correction

            // // write rest of config settings
            // conn.write(0x60, hpout_dly)?;
            // conn.write(0x60, hpout_outp_rmv_short)?;


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
