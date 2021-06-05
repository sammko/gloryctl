use std::{convert::TryFrom, str::FromStr};

use anyhow::{anyhow, Result};
use arrayvec::ArrayVec;
use bitflags::bitflags;
use hex::FromHex;
use hidapi::{HidApi, HidDevice};
use num_enum::TryFromPrimitive;

use crate::protocol::{decode, encode};

// Glorious Model O
const ID_VENDOR: u16 = 0x258a;
const ID_PRODUCT: u16 = 0x0036;
const CONTROL_IF: i32 = 1;
const HW_REPORT_MSG: u8 = 5;
const HW_REPORT_DATA: u8 = 4;
const HW_CMD_VER: u8 = 1;
const HW_CMD_CONF: u8 = 0x11;
const HW_CMD_MAP: u8 = 0x12;
const HW_CONF_WRITE_MAGIC: u8 = 0x7b;
const HW_MAP_WRITE_MAGIC: u8 = 0x50;

pub type DataReport = [u8; 520];

#[derive(Debug, Copy, Clone, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl FromStr for Color {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let buffer = <[u8; 3]>::from_hex(s)?;
        Ok(Self {
            r: buffer[0],
            g: buffer[1],
            b: buffer[2],
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum DpiValue {
    // probably use enums instead of u8
    Double(u8, u8),
    Single(u8),
}

#[derive(Debug, Copy, Clone)]
pub struct DpiProfile {
    pub enabled: bool,
    pub value: DpiValue,
    pub color: Color,
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Clone, Copy)]
#[repr(u8)]
pub enum PollingRate {
    Hz125 = 1,
    Hz250 = 2,
    Hz500 = 3,
    Hz1000 = 4,
}

pub mod rgb {
    use num_enum::TryFromPrimitive;

    use self::params::{
        Breathing, ConstantRgb, Glorious, Random, Rave, SeamlessBreathing, SingleBreathing,
        SingleColor, Tail, Wave,
    };
    use super::Color;

    #[derive(Debug, Eq, PartialEq, TryFromPrimitive, Clone, Copy)]
    #[repr(u8)]
    pub enum Effect {
        Off = 0,
        Glorious = 1,
        SingleColor = 2,
        Breathing = 3,
        Tail = 4,
        SeamlessBreathing = 5,
        ConstantRgb = 6,
        Rave = 7,
        Random = 8,
        Wave = 9,
        SingleBreathing = 10,
    }

    #[derive(Debug)]
    pub struct EffectParameters {
        pub glorious: Glorious,
        pub single_color: SingleColor,
        pub breathing: Breathing,
        pub tail: Tail,
        pub seamless_breathing: SeamlessBreathing,
        pub constant_rgb: ConstantRgb,
        pub rave: Rave,
        pub random: Random,
        pub wave: Wave,
        pub single_breathing: SingleBreathing,
    }

    // TODO: These should eventually be enums
    pub type Direction = u8;
    pub type Speed = u8;
    pub type Brightness = u8;

    #[allow(dead_code)]
    pub mod params {
        use super::{Brightness, Color, Direction, Speed};
        use arrayvec::ArrayVec;

        #[derive(Debug)]
        pub struct Glorious {
            pub speed: Speed,
            pub direction: Direction,
        }

        #[derive(Debug)]
        pub struct SingleColor {
            pub brightness: Brightness,
            pub color: Color,
        }

        #[derive(Debug)]
        pub struct Breathing {
            pub speed: Speed,
            pub count: u8,
            pub colors: ArrayVec<[Color; 7]>,
        }

        #[derive(Debug)]
        pub struct Tail {
            pub speed: Speed,
            pub brightness: Brightness,
        }

        #[derive(Debug)]
        pub struct SeamlessBreathing {
            pub speed: Speed,
        }

        #[derive(Debug)]
        pub struct ConstantRgb {
            pub colors: ArrayVec<[Color; 6]>,
        }

        #[derive(Debug)]
        pub struct Rave {
            pub speed: Speed,
            pub brightness: Brightness,
            pub colors: ArrayVec<[Color; 2]>,
        }

        #[derive(Debug)]
        pub struct Random {
            pub speed: Speed,
        }

        #[derive(Debug)]
        pub struct Wave {
            pub speed: Speed,
            pub brightness: Brightness,
        }

        #[derive(Debug)]
        pub struct SingleBreathing {
            pub speed: Speed,
            pub color: Color,
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub header: ArrayVec<[u8; 9]>,
    pub sensor_id: u8,
    pub dpi_axes_independent: bool,
    pub polling_rate: PollingRate,
    pub dpi_current_profile: u8,
    pub dpi_profile_count: u8,
    pub dpi_profiles: ArrayVec<[DpiProfile; 8]>,
    pub rgb_current_effect: rgb::Effect,
    pub rgb_effect_parameters: rgb::EffectParameters,
    pub unknown: (ArrayVec<[u8; 12]>, u8),
    pub lod: u8,
}

impl Config {
    pub fn from_raw(raw: &DataReport) -> Result<Config> {
        decode::config_report(raw)
            .map(|(_, c)| c)
            .map_err(|_| anyhow::Error::msg("Failed to parse config report"))

        // decode::config_report(raw)
        //     .map(|(_, c)| c)
        //     .map_err(|e| From::from(e.map_input(|i| i.to_owned())))
    }

    pub fn to_raw(&self) -> DataReport {
        encode::config_report(self)
    }

    pub fn fix_profile_count(&mut self) {
        self.dpi_profile_count = self.dpi_profiles.iter().filter(|p| p.enabled).count() as u8;
    }
}

bitflags! {
    pub struct Modifier: u8 {
        const CTRL  = 0x01;
        const SHIFT = 0x02;
        const ALT   = 0x04;
        const SUPER = 0x08;
    }
}

impl TryFrom<u8> for Modifier {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_bits(value).ok_or(())
    }
}

bitflags! {
    pub struct MouseButton: u8 {
        const LEFT    = 0x01;
        const RIGHT   = 0x02;
        const MIDDLE  = 0x04;
        const BACK    = 0x08;
        const FORWARD = 0x10;
    }
}

impl TryFrom<u8> for MouseButton {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_bits(value).ok_or(())
    }
}

bitflags! {
    pub struct MediaButton: u32 {
        const HOME_PAGE    = 0x000002;
        const MEDIA_PLAYER = 0x000100;
        const EXPLORER     = 0x000200;
        const EMAIL        = 0x001000;
        const CALCULATOR   = 0x002000;
        const NEXT         = 0x010000;
        const PREVIOUS     = 0x020000;
        const STOP         = 0x040000;
        const PLAY_PAUSE   = 0x080000;
        const MUTE         = 0x100000;
        const VOLUME_UP    = 0x400000;
        const VOLUME_DOWN  = 0x800000;
    }
}

pub mod buttonmap {
    use super::{MediaButton, Modifier, MouseButton};

    #[derive(Debug)]
    #[repr(u8)]
    pub enum DpiSwitch {
        Cycle = 0,
        Up = 1,
        Down = 2,
    }

    #[derive(Debug)]
    pub enum MacroMode {
        Burst(u8),
        RepeatUntilRelease,
        RepeatUntilAnotherPress,
    }

    #[derive(Debug)]
    pub enum ButtonAction {
        MouseButton(MouseButton),
        Scroll(u8),
        RepeatButton { which: u8, interval: u8, count: u8 },
        DpiSwitch(DpiSwitch),
        DpiLock(u8),
        MediaButton(MediaButton),
        KeyboardShortcut { modifiers: Modifier, key: u8 },
        Disabled,
        Macro(u8, MacroMode),
    }
}

pub mod macros {
    use super::{Modifier, MouseButton};

    #[repr(u8)]
    pub enum EventType {
        Keyboard(u8),
        Modifier(Modifier),
        Mouse(MouseButton),
    }

    pub enum State {
        Up,
        Down,
    }

    pub struct Event {
        pub state: State,
        pub evtype: EventType,
        pub duration: u16,
    }

    pub struct Macro {
        pub bank_number: u8,
        pub events: Vec<Event>,
    }
}

pub type ButtonMapping = [buttonmap::ButtonAction; 6];
pub struct GloriousDevice {
    pub hiddev: HidDevice,
}

impl GloriousDevice {
    pub fn open_first(hid: &HidApi) -> Result<GloriousDevice> {
        let devinfo = hid
            .device_list()
            .filter(|dev| {
                dev.product_id() == ID_PRODUCT
                    && dev.vendor_id() == ID_VENDOR
                    && dev.interface_number() == CONTROL_IF
            })
            .next()
            .ok_or(anyhow!("Could not find a supported device."))?;
        let dev = devinfo.open_device(hid)?;
        let gdev = GloriousDevice { hiddev: dev };
        return Ok(gdev);
    }

    pub fn read_fw_version(&self) -> Result<String> {
        let mut buf = [HW_REPORT_MSG, HW_CMD_VER, 0, 0, 0, 0];
        self.hiddev.send_feature_report(&buf)?;
        self.hiddev.get_feature_report(&mut buf)?;
        decode::version(&buf)
            .map(|(_, c)| c.to_string())
            .map_err(|_| anyhow::Error::msg("Failed to parse firmware version"))
    }

    fn read_data(&self, cmd: u8) -> Result<DataReport> {
        let req = [HW_REPORT_MSG, cmd, 0, 0, 0, 0];
        self.hiddev.send_feature_report(&req)?;
        let mut buf = [0; 520];
        buf[0] = HW_REPORT_DATA;
        self.hiddev.get_feature_report(&mut buf)?;
        return Ok(buf);
    }

    pub fn read_config_raw(&self) -> Result<DataReport> {
        self.read_data(HW_CMD_CONF)
    }

    pub fn read_buttonmap_raw(&self) -> Result<DataReport> {
        self.read_data(HW_CMD_MAP)
    }

    pub fn read_config(&self) -> Result<Config> {
        self.read_config_raw().map(|c| Config::from_raw(&c))?
    }

    pub fn read_buttonmap(&self) -> Result<ButtonMapping> {
        let raw = self.read_buttonmap_raw()?;
        decode::buttonmap(&raw)
            .map(|(_, c)| c)
            .map_err(|_| anyhow::Error::msg("Failed to parse button map"))
    }

    fn send_data(&mut self, cmd: u8, magic3: u8, data: &DataReport) -> Result<()> {
        let req = [HW_REPORT_MSG, cmd, 0, 0, 0, 0];
        self.hiddev.send_feature_report(&req)?;
        let mut datacpy = data.to_owned();
        datacpy[3] = magic3;
        self.hiddev.send_feature_report(&datacpy)?;
        // The mouse sometimes gets confused when reading the config right after
        // writing it. Wait a bit just in case. 10ms seems to be probably enough,
        // doing 20 for good measure.
        std::thread::sleep(std::time::Duration::from_millis(20));
        Ok(())
    }

    pub fn send_config_raw(&mut self, data: &DataReport) -> Result<()> {
        self.send_data(HW_CMD_CONF, HW_CONF_WRITE_MAGIC, data)
    }

    pub fn send_buttonmap_raw(&mut self, data: &DataReport) -> Result<()> {
        self.send_data(HW_CMD_MAP, HW_MAP_WRITE_MAGIC, data)
    }

    pub fn send_config(&mut self, conf: &Config) -> Result<()> {
        let x = conf.to_raw();
        self.send_config_raw(&x)
    }

    pub fn send_buttonmap(&mut self, map: &ButtonMapping) -> Result<()> {
        let x = encode::buttonmap(&map);
        self.send_config_raw(&x)
    }
}
