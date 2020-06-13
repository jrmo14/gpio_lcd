use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

use clap::{crate_authors, crate_version, App, Arg};

use lcd::Lcd;

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

    let lcd = match Lcd::new(
        2,
        16,
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

    lcd.clear();

    // match lcd.set_cursor(0, 0) {
    //     Ok(_) => {}
    //     Err(e) => return Err(format!("{}", e)),
    // };

    match lcd.print("HI THERE!") {
        Ok(_) => {}
        Err(e) => return Err(format!("{}", e)),
    }

    match lcd.set_cursor(1, 0) {
        Ok(_) => {}
        Err(e) => return Err(format!("{}", e)),
    }

    match lcd.print("Testing LCD.") {
        Ok(_) => {}
        Err(e) => return Err(format!("{}", e)),
    }

    lcd.clear();
    while true {
        for i in 0..128 as u8 {
            lcd.clear();
            lcd.set_cursor(0, 0);
            lcd.write(i);
            lcd.set_cursor(1, 0);
            lcd.print(i.to_string().as_str());
            sleep(Duration::from_millis(100));
        }
    }
    Ok(())
}
