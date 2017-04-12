use stm32f7::board::sai::{Sai,Bclrfr};
use stm32f7::board::rcc::Rcc;
use stm32f7::board::sai;
use embedded::interfaces::gpio::Gpio;
use i2c;
use bit_field::BitField;
use stm32f7::system_clock;

const WM8994_ADDRESS: i2c::Address = i2c::Address::bits_7(0b0011010);

pub struct Sound {
    written: u32,
}

impl Sound {

    fn init_clock(sai: &mut Sai, i2c_3: &mut i2c::I2C, rcc: &mut Rcc) {

        // println!("getting bit of pllsrc {}", rcc.pllcfgr.read().pllsrc());
        // println!("getting factor n of pllsaicfgr {}", rcc.pllsaicfgr.read().pllsain());
        // TODO if we are using PLLSAIQ to drive this unit, is PLLI2S relevant?
        // // Disable the PLLI2S
        // rcc.cr.update(|r| r.set_plli2son(false));
        // println!("before wait for pllclock");
        // while rcc.cr.read().plli2srdy() {}
        // println!("after wait for pllclock");

        // disable PLLSAIQ
        rcc.cr.update(|r| r.set_pllsaion(false));
        while rcc.cr.read().pllsairdy() {}

        // disable PLLI2S, because SAI2SEL will be written later
        rcc.cr.update(|r| r.set_plli2son(false));
        while rcc.cr.read().plli2srdy() {}

        rcc.dkcfgr1.update(|r| {
            r.set_sai2sel(0b01); // SET PLLSAI_Q / PLLSAIDIVQ
        });
        // println!("clock config");

        // In case PLLSOURCE is HSE
        // then PLL_(VCO INPUT) = PLLSRC/PLLM
        // let vcoinput = 25000000 / u32::from(rcc.pllcfgr.read().pllm());
        // let freq_vcoclock = vcoinput * u32::from(rcc.pllsaicfgr.read().pllsain());
        // println!("pllsaiq {}", rcc.pllsaicfgr.read().pllsaiq());
        // let freq_pllsaiq = freq_vcoclock / u32::from(rcc.pllsaicfgr.read().pllsaiq());
        // println!("pllsaidivq {}", rcc.dkcfgr1.read().pllsaidivq());
        // pllsaidivq stored with offset
        // let frequency = freq_pllsaiq / (u32::from(rcc.dkcfgr1.read().pllsaidivq() + 1));


        // println!("Our SAI CLK Frequency seems to be {}", frequency);

        rcc.plli2scfgr.update(|r| {
            r.set_plli2sn(344);
            r.set_plli2sq(7);
        });

        rcc.dkcfgr1.update(|r| r.set_plli2sdiv(1 - 1));



        let audio_frequency = 48000;

        {
            let mckdiv = {
                // Configure Master Clock using the following formula :
                // MCLK_x = SAI_CK_x / (MCKDIV[3:0] * 2) with MCLK_x = 256 * FS
                // FS = SAI_CK_x / (MCKDIV[3:0] * 2) * 256
                // MCKDIV[3:0] = SAI_CK_x / FS * 512

                // Get SAI clock source based on Source clock selection from RCC
                let freq = {
                    // Configure the PLLSAI division factor
                    // PLLSAI_VCO Input  = PLL_SOURCE/PLLM
                    // In Case the PLL Source is HSE (External Clock)
                    let vcoinput = 25000000 / u32::from(rcc.pllcfgr.read().pllm());

                    // PLLSAI_VCO Output = PLLSAI_VCO Input * PLLSAIN
                    // SAI_CLK(first level) = PLLSAI_VCO Output/PLLSAIQ
                    let tmpreg = u32::from(rcc.pllsaicfgr.read().pllsaiq());
                    let frequency = (vcoinput * u32::from(rcc.pllsaicfgr.read().pllsain())) / tmpreg;

                    // SAI_CLK_x = SAI_CLK(first level)/PLLSAIDIVQ
                    let tmpreg = u32::from(rcc.dkcfgr1.read().pllsaidivq()) + 1;
                    frequency / tmpreg
                };

                // (saiclocksource x 10) to keep Significant digits
                let tmpclock = (freq * 10) / (audio_frequency * 512);

                let mckdiv = tmpclock / 10;

                // Round result to the nearest integer
                if (tmpclock % 10) > 8 {
                    mckdiv + 1
                } else {
                    mckdiv
                }
            };

            sai.acr1.update(|r| r.set_mcjdiv(mckdiv as u8));
            println!("Set MCKDIV to {}", mckdiv);
        }






        // enable PLLSAIQ
        rcc.cr.update(|r| r.set_pllsaion(true));
        while !rcc.cr.read().pllsairdy() {}

        // enable PLLI2S
        rcc.cr.update(|r| r.set_plli2son(true));
        while !rcc.cr.read().plli2srdy() {}

    }

    pub fn init(sai: &mut Sai, i2c_3: &mut i2c::I2C, rcc: &mut Rcc, gpio: &mut Gpio) -> Self {

        sai.acr1.update(|r| r.set_saiaen(false));
        sai.bcr1.update(|r| r.set_saiben(false));

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
        sai.acr2.update(|r| r.set_fflus(true)); // fifo_flush

        Self::init_clock(sai, i2c_3, rcc);

        rcc.dkcfgr1.update(|r| r.set_sai2sel(0)); // sai2_clock_source pllsaiq/pllsaiq

        rcc.apb2enr.update(|r| r.set_sai2en(true));

        sai.gcr.update(|r| {
            r.set_syncout(0b00);
        });


        sai.acr1.update(|r| {
            // clock must be present
            r.set_mode(0b00); // PROBABLY this if not 0b01
            r.set_mono(false);
            r.set_lsbfirst(false);
            r.set_ds(0b100);
            r.set_prtcfg(0b00);
            r.set_mono(false);
            r.set_nodiv(false);
            r.set_out_dri(false);
            r.set_syncen(0b00);
            r.set_ckstr(true); // TODO seems to have no effect for now
            // r.set_mcjdiv(0b10);
        });


        // let mckdiv = 0b0010;
        // sai.acr1.update(|r| r.set_mcjdiv(mckdiv as u8));
        // configure frame
        {
            let mut afrcr = sai::Afrcr::default();
            afrcr.set_frl(64 - 1); // frame_length
            afrcr.set_fsall(32 - 1); // sync_active_level_length NOTE one ought to be enough (TDM DSP Mode B)
            afrcr.set_fsdef(false); // frame_sync_definition
            afrcr.set_fspol(true); // frame_sync_polarity
            afrcr.set_fsoff(false); // frame_sync_offset
            sai.afrcr.write(afrcr);
        }


        // configure slots
        {
            let mut aslotr = sai::Aslotr::default();
            let mut slots = 0b0011 as u16;
            aslotr.set_nbslot(0b0001);
            aslotr.set_slotsz(0b01); // Set explicitly
            aslotr.set_sloten(slots);
            aslotr.set_fboff(0); // offset of data in slot
            sai.aslotr.write(aslotr);
        }

        sai.acr2.update(|r| {
            r.set_mute(false);
            r.set_comp(0b00);
            r.set_tris(true);
        });

        sai.acr1.update(|r| r.set_saiaen(true));
        println!("before wait for saienable");
        while !sai.acr1.read().saiaen() {}
        println!("after wait for saienable");

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


        self::config_gpio(gpio);

        let ret = i2c_3.connect::<u16, _>(WM8994_ADDRESS, |mut conn| {

            system_clock::wait(10);
            // read and check device family ID
            assert_eq!(conn.read(0).ok(), Some(0x8994));
            // reset device
            conn.write(0, 0)?;

            // wm8994 Errata Work-Arounds
            conn.write(0x102, 0x0003)?;
            conn.write(0x817, 0x0000)?;
            conn.write(0x102, 0x0000)?;

            // Enable VMID soft start (fast), Start-up Bias Current Enabled
            conn.write(0x39, 0x006C)?;

            // Enable bias generator, Enable VMID
            conn.write(0x01, 0x0003)?;

            system_clock::wait(50);

            {
                let mut bits = 0x0;
                conn.write(0x02, bits)?;
            }


            // PWR
            // {
            //     let mut bits = 0b010;
            //     conn.write(0x0001, bits)?;
            // }


            // AIF1DAC1X
            {
                let bits = 0x0303;
                conn.write(0x0005, bits);
            }

            // AIF1DACXL_TO_DAC1L
            {
                let mut bits = 0x01;
                conn.write(0x0601, bits);
                conn.write(0x602, bits);
            }

            //Disable AF1 Timeslot 1 Mixer Path
            // {
            //     let bits = 0x0;
            //     conn.write(0x604, bits)?;
            //     conn.write(0x605, bits)?;
            // }

            // Set frequency
            {
                // let mut bits = 0b1010;
                // conn.write(0x210, bits)?;
            }

            // AIF1 Word Length
            {
                let mut bits = 0x0;
                bits.set_bit(7, true); // In DPSMOde: Select mode B
                bits.set_bit(4, true);
                bits.set_bit(3, true);
                conn.write(0x0300, bits)?;
            }

            // Enable slave mode
            {
                let mut bits = 0x0;
                bits.set_bit(15, true);
                conn.write(0x0302, bits)?;
            }

            // Enable CORE AIF1 Clock
            {
                let mut bits = 0x0A;
                conn.write(0x0208, bits);
            }

            // AIF1CLK EN
            {
                let mut bits = 0x1;
                bits.set_bit(2, false);
                conn.write(0x0200, bits);
            }


            // For some reason another one of the bias generator
            // {
            //     let mut bits = 0x3003;
            //     conn.write(0x01, bits)?;
            // }

            // Class W Envelope Tracking???? NOTE disabled envelope tracking,
            // prior write request (look in git) did not matc any specified bit in Reg
            {
                let mut bits = 0x01;
                conn.write(0x51, bits)?;
            }

            // Manually initiating startup sequence of headphones
            {
                let seq_number = 0x8100;
                conn.write(0x0110, seq_number)?;
                system_clock::wait(300);
                // let power_mgnt_reg_1 = 0x0 |  0x0303 | 0x0003;
                // conn.write(0x1, power_mgnt_reg_1)?;

                // /* Add Delay */
                // system_clock::wait(5);

                // /* Enable HPOUT1 (Left) and HPOUT1 (Right) intermediate stages */
                // conn.write(0x60, 0x0022)?;

                // /* Enable Charge Pump */
                // conn.write(0x4C, 0x9F25)?;

                // /* Add Delay */
                // system_clock::wait(5);

                // /* Select DAC1 (Left) to Left Headphone Output PGA (HPOUT1LVOL) path */
                // // conn.write( 0x2D, 0x0001)?;

                // /* Select DAC1 (Right) to Right Headphone Output PGA (HPOUT1RVOL) path */
                // // conn.write( 0x2E, 0x0001)?;

                // /* Enable Left Output Mixer (MIXOUTL), Enable Right Output Mixer (MIXOUTR) */
                // /* idem for SPKOUTL and SPKOUTR */
                // // conn.write( 0x03, 0x0030 | 0x0300);

                // /* Enable DC Servo and trigger start-up mode on left and right channels */
                // conn.write( 0x54, 0x0033);

                // /* Add Delay */
                // system_clock::wait(200);

                // /* Enable HPOUT1 (Left) and HPOUT1 (Right) intermediate and output stages. Remove clamps */
                // conn.write( 0x60, 0x00EE)?;
            }


            // USE DAC1 directly (no MIXER)
                {
                    let mut bits = 0x0;
                    bits.set_bit(8, true);
                    conn.write(0x2D, bits)?;
                    conn.write(0x2E, bits)?;
                }


            // Doing unmutes
            {
                /* Unmute DAC 1 (Left) */
                conn.write(0x610, 0x00C0)?;

                /* Unmute DAC 1 (Right) */
                conn.write(0x611, 0x00C0)?;

                /* Unmute the AIF1 Timeslot 0 DAC path */
                conn.write(0x420, 0x0000)?;

                /* Unmute DAC 2 (Left) */
                conn.write(0x612, 0x00C0)?;

                /* Unmute DAC 2 (Right) */
                conn.write(0x613, 0x00C0)?;

                /* Unmute the AIF1 Timeslot 1 DAC2 path */
                // conn.write(0x422, 0x0000)?;

                /* Volume Control */
                // wm8994_SetVolume(DeviceAddr, Volume);
                {
                    let mut ldata = 0x2d;
                    conn.write(0x1C, 0x3F | 0x140)?;

                    /* Right Headphone Volume */
                    conn.write(0x1D, 0x3F | 0x140)?;
                }
            }

            system_clock::wait(10);
            println!("Status WM8994 Register: {:b}", conn.read(0x212)?);
            Ok(())
        });


        Sound {
        written: 0
        }
    }

    pub fn tick(&mut self) {
        // println!("Sound Tick");
    }


    #[inline(never)]
    pub fn put_data(&mut self, sai: &mut Sai, i2c_3: &mut i2c::I2C, data: u32) -> u32 {

        let mut fifo_data = data;

        while sai.asr.read().flvl() != 0b101 {

            // println!("writing in FIFO");
            // sai.adr.update(|reg| reg.set_data(fifo_data & 0xFFFFFFFF));
            let mut actual_data = sai::Adr::default();
            actual_data.set_data(fifo_data);
            let zero_data = sai::Adr::default();
            sai.adr.write(actual_data);
            sai.adr.write(zero_data);
            sai.adr.write(zero_data);
            sai.adr.write(zero_data);
            self.written = self.written.wrapping_add(32 * 4);
            fifo_data = fifo_data.wrapping_add(10000);

            // self.written = self.written.wrapping_add(32);

            // fifo_data = fifo_data.wrapping_add(10000);
            // sai.acr2.update(|r| r.set_fflus(true));
            // sai.acr1.update(|r| r.set_saiaen(false));
            // sai.acr1.update(|r| r.set_saiaen(true));

        }
        // println!("FIFO full, {} written", self.written);
        self.written = 0;
        return fifo_data;
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
        // let sai2_sd_b = (PortG, Pin10);

        // let pins = [sai2_fs_a, sai2_sck_a, sai2_sd_a, sai2_mclk_a, sai2_sd_b];
        let pins = [sai2_fs_a, sai2_sck_a, sai2_mclk_a, sai2_sd_a];
        gpio.to_alternate_function_all(&pins,
                                       AlternateFunction::AF10,
                                       OutputType::PushPull,
                                       OutputSpeed::High,
                                       Resistor::NoPull)
            .unwrap();
    }
