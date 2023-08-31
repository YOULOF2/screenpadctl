use serde_derive::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(PartialEq)]
enum ScreenState {
    On,
    Off,
    Dim,
}

#[derive(Serialize, Deserialize)]
struct Config {
    positive_increment: i16,
    negative_increment: i16,
}

impl ::std::default::Default for Config {
    fn default() -> Self {
        Self {
            positive_increment: 15,
            negative_increment: -15,
        }
    }
}

fn print_error(text: &str) {
    println!("\x1b[91mError: {}\x1b[0m", text);
}

fn print_success(text: &str) {
    println!("\x1b[92mSuccess: {}\x1b[0m", text);
}

const BRIGHTNESS_CTRL_FILE: &str = "/sys/class/leds/asus::screenpad/brightness";

fn get_brightness() -> i16 {
    let mut brightness_string =
        fs::read_to_string(BRIGHTNESS_CTRL_FILE).expect("Cannot open control file");

    if brightness_string.ends_with('\n') {
        brightness_string.pop();
    }

    brightness_string
        .parse::<i16>()
        .expect("Cannot convert string to int")
}

/// Overwite brightness
fn overwrite_brightness(value: i16) {
    fs::write(BRIGHTNESS_CTRL_FILE, value.to_string()).expect("Cannot write new value to file");
}

/// increment brightness by +/-ve value
fn increment_brightness(value: i16) {
    let current_brightness = get_brightness();

    if (current_brightness + value) <= 255 && (current_brightness + value) >= 0 {
        overwrite_brightness(current_brightness + value);
    }
}

const BRIGHTNESS_BACKUP_FILE: &str = "~/.local/share/brightness_backup";

/// Store current brightness in file
fn backup_brightness() {
    let current_brightness = get_brightness();

    if !Path::new(BRIGHTNESS_BACKUP_FILE).exists() {
        fs::File::create(BRIGHTNESS_BACKUP_FILE).expect("Cannot create backup file");
    }

    fs::write(BRIGHTNESS_BACKUP_FILE, current_brightness.to_string())
        .expect("Cannot write to backup file");
}

/// restore previous brightness value
fn restore_brightness() -> i16 {
    let mut prev_brightness =
        fs::read_to_string(BRIGHTNESS_BACKUP_FILE).expect("Cannot open backup file");

    if prev_brightness.ends_with('\n') {
        prev_brightness.pop();
    }

    prev_brightness
        .parse::<i16>()
        .expect("Cannot convert string to int")
}

/// Get current state of display
/// 0 -> off
/// 1 -> on
/// 2 -> dim
fn screen_state() -> ScreenState {
    let current_brightness = get_brightness();

    return match current_brightness {
        0 => ScreenState::Off,
        1 => ScreenState::Dim,
        _ => ScreenState::On,
    };
}

fn main() {
    let mut cfg: Config = confy::load("screenpadctl", None).expect("Cannot Create Config File");

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_error("Specify argument\nuse `help` for usage details");
        return;
    }

    let current_state = screen_state();

    match args[1].as_str() {
        "b" => println!("Current Brightness is {}", get_brightness()),

        "bup" => {
            increment_brightness(cfg.positive_increment);
            print_success("Brightness up");
        }
        "bdown" => {
            increment_brightness(cfg.negative_increment);
            print_success("Brightness down");
        }
        "bconfig" => {
            if args.len() <= 2 {
                print_error("Specify which increment value to change. Use [pos/neg] <value>");
                return;
            }

            if args.len() <= 3 {
                print_error("Specify increment value");
                return;
            }

            let value = args[3]
                .parse::<i16>()
                .expect("Cannot convert string to int");
            let operation = &args[2];

            match operation.as_str() {
                "pos" => cfg.positive_increment = value,
                "neg" => cfg.negative_increment = value,
                _ => {
                    print_error("Enter valid increment value to change");
                    return;
                }
            }

            let _ = confy::store("screenpadctl", None, cfg);
            print_success(format!("Set {} increment to {}", operation, value).as_str());
        }
        "bset" => {
            if args.len() <= 2 {
                print_error(
                    "Specifiy int between [0->255] inclusive to set the brightness manually",
                );
                return;
            }

            let value = match args[2].parse::<i16>() {
                Ok(value) => value,
                Err(_) => {
                    print_error("Enter a valid int between [0->255] inclusive");
                    return;
                }
            };

            if value > 255 || value < 0 {
                print_error("Int out of range. Brightness is between [0->255] inclusive");
                return;
            }

            overwrite_brightness(value);
            print_success(format!("Set brightness to {}", value).as_str());
        }

        "on" => {
            if current_state == ScreenState::On {
                print_error("Screen is already on");
                return;
            }
            overwrite_brightness(restore_brightness());
            print_success("Screen on");
        }

        "off" => {
            if current_state == ScreenState::Off {
                print_error("Screen is already off");
                return;
            }
            backup_brightness();
            overwrite_brightness(0);
            print_success("Screen off");
        }
        "toggle" => {
            if current_state == ScreenState::On {
                backup_brightness();
                overwrite_brightness(0);
                print_success("Toggle screen off");
            } else if current_state == ScreenState::Off {
                overwrite_brightness(restore_brightness());
                print_success("Toggle screen on");
            }
        }
        "dim" => {
            if current_state == ScreenState::Dim {
                return;
            }

            if current_state == ScreenState::On {
                backup_brightness();
            }

            overwrite_brightness(1);
            print_success("Dim Screen");
        }
        "cycle" => {
            // on -> dim -> off
            match current_state {
                // on to dim
                ScreenState::On => {
                    backup_brightness();
                    overwrite_brightness(1);
                    print_success("Cycle on -> dim");
                }
                // dim to off
                ScreenState::Dim => {
                    overwrite_brightness(0);
                    print_success("Cycle dim -> off");
                }
                // off to on
                ScreenState::Off => {
                    overwrite_brightness(restore_brightness());
                    print_success("Cycle off -> on");
                }
            }
        }

        "help" => println!(
            "Usage details:
        Print current brightness: `b`
        Config brightness increment: `bconfig [pos/neg] <value>`
        Brightness control: `bup`, `bdown`, `bset <value>`
        Power control: `on`, `off`, `dim`
        Special power control modes: 
            `toggle`: toggle between on and off
            `cycle`: cycle between [on -> dim -> off] (loops)
"
        ),

        _ => print_error("Invalid Argument\nUse `help` command"),
    }
}
