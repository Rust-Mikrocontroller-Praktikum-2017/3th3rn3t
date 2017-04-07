use stm32f7::board;

pub struct Sound {}

impl Sound {

    pub fn init(sai: &mut board::sai::Sai, rcc: &mut board::rcc::Rcc) -> Self {

        sai.acr1.update(|r| {
            // clock must be present
            r.set_saiaen(true);
            r.set_mode(0b00);
        });
        Sound {}
    }

    pub fn tick(&mut self) {
        println!("Sound Tick");
    }
}
