use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;
use std::str;

use clap::{crate_authors, crate_version, App, Arg};
use gpio_lcd::lcd::LcdDriver;
use gpio_lcd::scheduler::{Job, ThreadedLcd};

fn main() -> Result<(), String> {
    let matches = App::new("Rust LCD Test")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Test program for LCD screen")
        .arg(
            Arg::with_name("chip")
                .short("c")
                .long("chip")
                .value_name("CHIP")
                .help("Sets the chip to use for GPIO")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("four_bit_mode")
                .short("f")
                .long("mode")
                .help("Sets the bit mode for the LCD panel")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("rs")
                .long("rs")
                .value_name("RS_PIN")
                .help("The pin to use for rs")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("rw")
                .long("rw")
                .value_name("RW_PIN")
                .help("The pin to use for rw")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("enable")
                .short("e")
                .long("enable")
                .value_name("ENABLE_PIN")
                .help("The pin to use for enable")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("data_pins")
                .short("d")
                .long("data_pins")
                .value_name("DATA_PINS")
                .help("The 8 data pins")
                .multiple(true)
                .required(true),
        )
        .get_matches();

    let data_pins_res: Vec<Result<u8, std::num::ParseIntError>> = matches
        .values_of("data_pins")
        .unwrap()
        .map(|p| u8::from_str(p))
        .collect();

    let mut data_pins = Vec::new();

    if data_pins_res.len() != 8 && data_pins_res.iter().any(|res| res.is_err()) {
        return Err("Wrong number of data pins passed".parse().unwrap());
    }
    data_pins_res.iter().for_each(|pin_res| {
        data_pins.push(pin_res.as_ref().unwrap());
    });

    let mut lcd = match LcdDriver::new(
        16,
        2,
        matches.value_of("chip").unwrap_or("/dev/gpiochip0"),
        matches.is_present("four_bit_mode"),
        u8::from_str(matches.value_of("rs").unwrap()).unwrap(),
        u8::from_str(matches.value_of("rw").unwrap_or("255")).unwrap(),
        u8::from_str(matches.value_of("enable").unwrap()).unwrap(),
        *data_pins[0],
        *data_pins[1],
        *data_pins[2],
        *data_pins[3],
        *data_pins[4],
        *data_pins[5],
        *data_pins[6],
        *data_pins[7],
    ) {
        Ok(lcd) => lcd,
        Err(e) => return Err(format!("{}", e)),
    };

    let thread_driver = ThreadedLcd::with_driver(lcd);
    let test_codes: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6];
    thread_driver.add_job(Job::new(
        format!("Test {}", str::from_utf8(&test_codes).unwrap()).as_str(),
        0,
        Option::from(Duration::from_millis(500)),
    ));

    sleep(Duration::from_secs(60 * 60));
    thread_driver.clear_jobs();
    thread_driver.clear_row(0);
    thread_driver.clear_row(1);

    Ok(())
}
