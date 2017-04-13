#![no_std]
#![no_main]

#![feature(asm)]
#![feature(const_fn)]
#![feature(alloc, collections)]

#[macro_use]
extern crate stm32f7_discovery as stm32f7;
extern crate r0;
extern crate bit_field;
#[macro_use]
extern crate collections;
extern crate alloc;
#[macro_use]
extern crate net;

#[macro_use]
use stm32f7::{random, audio, ethernet, sdram, system_clock, board, embedded, touch, i2c, lcd};

#[macro_use]
mod semi_hosting;
mod font;
mod graphics;
mod sound;

use random::{Rng,ErrorType};
use graphics::Graphics;

use collections::string::String;
use collections::BTreeMap;

use net::ethernet::{EthernetAddress, EthernetPacket, EthernetKind, EtherType};
use net::ipv4::{Ipv4Address, Ipv4Packet, Ipv4Kind, IpProtocol};
use net::TxPacket;
use net::arp;
use net::udp::{UdpPacket, UdpKind};
use net::dns::{self, DnsPacket, };
use net::dhcp::{self, DhcpPacket, DhcpType};
use net::icmp::IcmpType;

use ethernet::EthernetDevice;

static sin440: [u16; 48000] = include!("sin.hex");

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

    let scb = stm32f7::cortex_m::peripheral::scb_mut();
    scb.cpacr.modify(|v| v | 0b1111 << 20);

    main(board::hw());
}

#[inline(never)]
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
        rng,
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
    i2c::init_pins_and_clocks(rcc, &mut gpio);
    let mut i2c_3 = i2c::init(i2c_3);

    let mut graphics = Graphics::init(ltdc, rcc, &mut gpio, &mut i2c_3);

    // original code - may be incompatible with our plans
    // audio::init_sai_2_pins(&mut gpio);
    // audio::init_sai_2(sai_2, rcc);
    // assert!(audio::init_wm8994(&mut i2c_3).is_ok());

    let mut led = gpio.to_output(led_pin,
                                 gpio::OutputType::PushPull,
                                 gpio::OutputSpeed::Low,
                                 gpio::Resistor::NoPull,)
        .expect("led pin already in use");


    let mut eth_device = ethernet::EthernetDevice::new(
        Default::default(),
        Default::default(),
        rcc,
        syscfg,
        &mut gpio,
        ethernet_mac,
        ethernet_dma
        );

    if let Err(e) = eth_device {
        println!("ethernet init failed: {:?}", e);
    } else {
        println!("ethernet init successful");
    }

    let mut random_gen = random::Rng::init(rng, rcc).expect("rng already enabled");

    let mut last_toggle_ticks = system_clock::ticks();

    graphics.prepare();

    let mut snd = sound::Sound::init(sai_2, &mut i2c_3, rcc, &mut gpio);

    loop {

        //println!("tick foobar");

        let ticks = system_clock::ticks();
        if (ticks - last_toggle_ticks)  > 1500 {
            let current_led_state = led.get();
            led.set(!current_led_state);
            last_toggle_ticks = ticks;
        }

        // println!("result from random.tick() {}", random.tick());
        //snd.tick();

        /*
         *if let Ok(number) = random_gen.poll_and_get() {
         *    snd.put_data(sai_2, &mut i2c_3, number);
         *} else {
         *    println!("No random data ready");
         *}
         */

        // this is the Ethernet tick
        if let Ok(ref mut eth_device) = eth_device {
            if let Err(err) = eth_device.handle_next_packet(&packets) {
                match err {
                    stm32f7::ethernet::Error::Exhausted => {}
                    e => {println!("err {:?}", e);}
                }
            }
        }

        //graphics.tick(&mut i2c_3);

    }
}

pub enum ParseResultType {
    Unknown,
    ARP,
    DHCP,
    ICMP,
    DNS,
    HTTP
}

pub enum ParseResultDirection {
    Request,
    Response
}

pub struct ParseResult {
    mac_src: Option<EthernetAddress>,
    mac_dst: Option<EthernetAddress>,
    ipv4_src: Option<Ipv4Address>,
    ipv4_dst: Option<Ipv4Address>,
    port_src: Option<u16>,
    port_dst: Option<u16>,
    pkt_type: ParseResultType,
    direction: Option<ParseResultDirection>,
    hostname: Option<String>
}

pub fn packets(data: &[u8], ipv4_addr: &mut Option<Ipv4Address>, requested_ipv4_addr: &mut Option<Ipv4Address>, arp_cache: &mut BTreeMap<Ipv4Address, EthernetAddress>) -> (ParseResult, Option<TxPacket>) {
    let mut parse_result = ParseResult {mac_src: None, mac_dst: None, ipv4_src: None, ipv4_dst: None, port_src: None, port_dst: None, pkt_type: ParseResultType::Unknown, direction: None, hostname: None};

    let eth_packet = net::parse(data).unwrap();

    // extract some high level packet information
    {
        let EthernetPacket {ref header, ref payload} = eth_packet;
        // TODO: does this copy? maybe use .clone()
        parse_result.mac_src = Some(header.src_addr);
        parse_result.mac_dst = Some(header.dst_addr);
        match *payload {
            EthernetKind::Ipv4(Ipv4Packet {header: ref ipv4_header, payload: ref ipv4_payload}) => {

                parse_result.ipv4_src = Some(ipv4_header.src_addr);
                parse_result.ipv4_dst = Some(ipv4_header.dst_addr);

                match *ipv4_payload {
                    Ipv4Kind::Udp(UdpPacket {header: ref udp_header, payload: ref udp_payload}) => {
                        parse_result.port_src = Some(udp_header.src_port);
                        parse_result.port_dst = Some(udp_header.dst_port);
                        
                        match *udp_payload {
                            UdpKind::Dhcp(DhcpPacket {mac, operation, .. }) => {

                                parse_result.pkt_type = ParseResultType::DHCP;

                                match operation {
                                    DhcpType::Offer { ip, dhcp_server_ip } => {
                                        parse_result.ipv4_dst = Some(ip);
                                    }
                                    DhcpType::Ack { ip } => {
                                        parse_result.ipv4_src = Some(ip);
                                    }
                                    _ => {}
                                }
                            }
                            UdpKind::Dns(DnsPacket {header: dns_header, hostname: _}) => {
                                parse_result.pkt_type = ParseResultType::DNS;
                                //parse_result.hostname = Some(hostname);
                            }
                            UdpKind::Unknown(_) => {
                                parse_result.pkt_type = ParseResultType::Unknown;
                            }
                        }
                    }
                    Ipv4Kind::Icmp(icmp) => {
                        parse_result.pkt_type = ParseResultType::ICMP;
                    }
                    Ipv4Kind::Unknown(_, _) => {
                        parse_result.pkt_type = ParseResultType::Unknown;
                    }
                }
            }
            EthernetKind::Arp(_) => {
                // TODO: maybe look at the ARP type to extract the direction, or IP addresses
                parse_result.pkt_type = ParseResultType::ARP;
            }
            EthernetKind::Unknown(_) => {
                parse_result.pkt_type = ParseResultType::Unknown;
            }
        }
    }

    // This uses methods of the driver to create reply packets
    if let Some(res) = EthernetDevice::handle_dhcp(&eth_packet, ipv4_addr, requested_ipv4_addr) {
        return (parse_result, res.unwrap());
    } else if let Some(res) = EthernetDevice::handle_arp(&eth_packet, ipv4_addr, arp_cache) {
        return (parse_result, res.unwrap());
    } else if let Some(res) = EthernetDevice::handle_icmp(&eth_packet, ipv4_addr, arp_cache) {
        return (parse_result, res.unwrap());
    } else {
        return (parse_result, None);
    }
}
