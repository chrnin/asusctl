use asus_nb::aura_modes::{AuraModes, BREATHING, STATIC, STROBE};
use log::{info, warn};
use serde_derive::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Read;

pub static LEDMODE_CONFIG_PATH: &str = "/etc/asusd/asusd-ledmodes.toml";

static HELP_ADDRESS: &str = "https://gitlab.com/asus-linux/asus-nb-ctrl";

pub struct LaptopBase {
    usb_product: String,
    supported_modes: Vec<u8>,
}

impl LaptopBase {
    pub fn usb_product(&self) -> &str {
        &self.usb_product
    }
    pub fn supported_modes(&self) -> &[u8] {
        &self.supported_modes
    }
}

pub fn match_laptop() -> Option<LaptopBase> {
    for device in rusb::devices().unwrap().iter() {
        let device_desc = device.device_descriptor().unwrap();
        if device_desc.vendor_id() == 0x0b05 {
            match device_desc.product_id() {
                0x1866 => {
                    let laptop = select_1866_device("1866".to_owned());
                    print_modes(&laptop.supported_modes);
                    return Some(laptop);
                }
                0x1869 => return Some(select_1866_device("1869".to_owned())),
                0x1854 => {
                    info!("Found GL753 or similar");
                    return Some(LaptopBase {
                        usb_product: "1854".to_string(),
                        supported_modes: vec![STATIC, BREATHING, STROBE],
                    });
                }
                _ => {}
            }
        }
    }
    None
}

fn select_1866_device(prod: String) -> LaptopBase {
    let dmi = sysfs_class::DmiId::default();
    let board_name = dmi.board_name().expect("Could not get board_name");
    let prod_family = dmi.product_family().expect("Could not get product_family");
    let prod_name = dmi.product_name().expect("Could not get product_name");

    info!("Product name: {}", prod_name.trim());
    info!("Board name: {}", board_name.trim());

    let mut laptop = LaptopBase {
        usb_product: prod,
        supported_modes: vec![],
    };

    if let Some(modes) = LEDModeGroup::load_from_config() {
        if let Some(led_modes) = modes.matcher(&prod_family, &board_name) {
            laptop.supported_modes = led_modes;
            return laptop;
        }
    }

    warn!(
        "Unsupported laptop, please request support at {}",
        HELP_ADDRESS
    );
    warn!("Continuing with minimal support");

    laptop
}

fn print_modes(supported_modes: &[u8]) {
    if !supported_modes.is_empty() {
        info!("Supported Keyboard LED modes are:");
        for mode in supported_modes {
            let mode = <&str>::from(&<AuraModes>::from(*mode));
            info!("- {}", mode);
        }
        info!(
            "If these modes are incorrect or missing please request support at {}",
            HELP_ADDRESS
        );
    } else {
        info!("No RGB control available");
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct LEDModeGroup {
    led_modes: Vec<LEDModes>,
}

impl LEDModeGroup {
    /// Consumes the LEDModes
    fn matcher(self, prod_family: &str, board_name: &str) -> Option<Vec<u8>> {
        for led_modes in self.led_modes {
            if prod_family.contains(&led_modes.prod_family) {
                for board in led_modes.board_names {
                    if board_name.contains(&board) {
                        info!("Matched to {} {}", led_modes.prod_family, board);
                        return Some(led_modes.led_modes);
                    }
                }
            }
        }
        None
    }

    fn load_from_config() -> Option<Self> {
        if let Ok(mut file) = OpenOptions::new().read(true).open(&LEDMODE_CONFIG_PATH) {
            let mut buf = String::new();
            if let Ok(l) = file.read_to_string(&mut buf) {
                if l == 0 {
                    warn!("{} is empty", LEDMODE_CONFIG_PATH);
                } else {
                    return Some(toml::from_str(&buf).unwrap_or_else(|_| {
                        panic!("Could not deserialise {}", LEDMODE_CONFIG_PATH)
                    }));
                }
            }
        }
        warn!("Does {} exist?", LEDMODE_CONFIG_PATH);
        None
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct LEDModes {
    prod_family: String,
    board_names: Vec<String>,
    led_modes: Vec<u8>,
}
