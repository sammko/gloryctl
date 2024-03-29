use std::convert::TryInto;
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};

use clap::{ArgEnum, Clap};
use gloryctl::macros::Event;
use gloryctl::{rgb::Effect, ButtonAction, Color, DpiValue, GloriousDevice};

#[derive(Clap)]
pub struct Opts {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Clap)]
enum Command {
    /// Dump the firmware version and Config
    Dump(Dump),
    /// Configure the button mapping
    Button(Buttons),
    /// Configure DPI profiles
    Dpi(Dpi),
    /// Configure macros
    Macro(Macro),
    /// Configure the RGB effect
    // This is weird due to https://github.com/clap-rs/clap/issues/2005
    Rgb {
        #[clap(subcommand)]
        rgbcmd: Rgb,
    },
}

#[derive(Clap)]
struct Dump {}

#[derive(Clap)]
#[clap(after_help = r"DISCUSSION:
    The format of a mapping is button:action-type[:action-params...]
    where button is a number from 1 to 6 and action-type:action-params]
    is one of the following:

    - disable
    - mouse:button (button is one of 'left', 'right', 'middle', 'back', 'forward')
    - scroll:amount (amount can also be 'up' and 'down', corresponding to 1 and -1)
    - repeat:button:count[:interval=50] (button is same as 'mouse', )
    - dpi:direction, direction is one of 'loop', 'up', 'down'
    - dpi-lock:value
    - media:key
    - macro:bank
    - keyboard:modifiers:key

    The provided mappings are always applied over the default configuration,
    not the current one. If no mappings are provided, the default configuration
    is used.

    The default configuration can be represented as:

    1:mouse:left 2:mouse:right 3:mouse:middle 4:mouse:back 5:mouse:forward 6:dpi:loop")]
struct Buttons {
    mappings: Vec<SingleButton>,
}

struct SingleButton {
    which: usize,
    action: ButtonAction,
}

impl FromStr for SingleButton {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (btn, act) = s
            .split_once(':')
            .context("Format: button:action-type[:action-params]")?;
        let which = usize::from_str(btn)?;
        let action = ButtonAction::from_str(act)?;
        Ok(Self { which, action })
    }
}

#[derive(Clap)]
#[clap(after_help = r"DISCUSSION:
    The mouse has support for 8 dpi profiles, of which each has a
    configured dpi value and a color (which is displayed on the LED
    on the bottom of the mouse. For example, to change the color
    of dpi profile number 3, you could use

        gloryctl dpi -c 00ffff 3

    At this point, it is not possible to enable or disable profiles.")] // TODO
struct Dpi {
    #[clap(possible_values = &["1", "2", "3", "4", "5", "6", "7", "8"])]
    which: usize,

    #[clap(short, long)]
    color: Option<Color>,

    #[clap(short, long)]
    dpi: Option<u16>,
    // TODO independent X and Y
}

#[derive(Clap)]
#[clap(after_help = r"DISCUSSION:
    This subcommand can be used to program macros. The first argument
    is the bank number. Following is a list of events. Each event has
    a format of state:type:key:duration.

    - state is either 'up' or 'down'
    - type is one of 'keyboard', 'modifier', 'mouse'
    - key takes on values depending on type, similar to button mappings
    - duration is in milliseconds, how long to pause before continuing")]
struct Macro {
    bank: u8,

    events: Vec<Event>,
}

#[derive(Clap)]
enum Rgb {
    /// Lighting disabled
    Off,
    /// Rotating rainbow (default for new mice)
    Glorious {
        #[clap(arg_enum, long, short)]
        direction: Option<Direction>,

        #[clap(arg_enum, long, short)]
        speed: Option<Speed>,
    },
    /// Single color
    Single {
        #[clap(long, short)]
        color: Option<Color>,

        #[clap(arg_enum, long, short)]
        brightness: Option<Brightness>,
    },
    /// Slowly cycles through the given list of colors
    Breathing {
        #[clap(arg_enum, long, short)]
        speed: Option<Speed>,

        //#[clap(long, short, max_values = 7)]
        // we are not using max_values here, because it
        // leads to confusing behaviour when more values are passed
        #[clap(long, short)]
        colors: Vec<Color>,
    },
    ///
    Tail {
        #[clap(arg_enum, long, short)]
        brightness: Option<Brightness>,

        #[clap(arg_enum, long, short)]
        speed: Option<Speed>,
    },
    /// Cycle through colors seamlessly
    SeamlessBreathing {
        #[clap(arg_enum, long, short)]
        speed: Option<Speed>,
    },
    /// Constant color for each of the six LEDs
    ConstantRgb {
        #[clap(long, short, number_of_values = 6)]
        colors: Vec<Color>,
    },
    /// Switching between two configured colors
    Rave {
        #[clap(arg_enum, long, short)]
        brightness: Option<Brightness>,

        #[clap(arg_enum, long, short)]
        speed: Option<Speed>,

        #[clap(long, short, number_of_values = 2)]
        colors: Vec<Color>,
    },
    /// Randomly changing colors
    Random {
        #[clap(arg_enum, long, short)]
        speed: Option<Speed>,
    },
    Wave {
        #[clap(arg_enum, long, short)]
        brightness: Option<Brightness>,

        #[clap(arg_enum, long, short)]
        speed: Option<Speed>,
    },
    /// Single color breathing
    SingleBreathing {
        #[clap(arg_enum, long, short)]
        speed: Option<Speed>,

        #[clap(long, short)]
        color: Option<Color>,
    },
}

#[derive(ArgEnum)]
enum Direction {
    Up,
    Down,
}

#[derive(ArgEnum)]
enum Speed {
    Slow,
    Medium,
    Fast,
}

#[derive(ArgEnum)]
enum Brightness {
    _0,
    _25,
    _50,
    _75,
    _100,
}

impl From<&Direction> for u8 {
    fn from(d: &Direction) -> u8 {
        match d {
            Direction::Up => 1,
            Direction::Down => 0,
        }
    }
}

impl From<&Speed> for u8 {
    fn from(s: &Speed) -> u8 {
        match s {
            Speed::Slow => 1,
            Speed::Medium => 2,
            Speed::Fast => 3,
        }
    }
}

impl From<&Brightness> for u8 {
    fn from(b: &Brightness) -> u8 {
        match b {
            Brightness::_0 => 0,
            Brightness::_25 => 1,
            Brightness::_50 => 2,
            Brightness::_75 => 3,
            Brightness::_100 => 4,
        }
    }
}

impl Dump {
    fn run(&self, dev: &mut GloriousDevice) -> Result<()> {
        dbg!(dev.read_fw_version()?);
        dbg!(dev.read_config()?);
        //dbg!(dev.read_buttonmap()?);
        Ok(())
    }
}

impl Buttons {
    fn run(&self, dev: &mut GloriousDevice) -> Result<()> {
        let mut map = gloryctl::DEFAULT_MAP;
        for b in &self.mappings {
            if b.which < 1 || b.which > 6 {
                return Err(anyhow!("Invalid button number {}", b.which));
            }
            let i = b.which - 1;
            map[i] = b.action;
        }
        dev.send_buttonmap(&map)
    }
}

impl Dpi {
    fn run(&self, dev: &mut GloriousDevice) -> Result<()> {
        let mut conf = dev.read_config()?;
        assert!(self.which >= 1 && self.which <= 8);
        let i = self.which - 1;
        let prof = &mut conf.dpi_profiles[i];

        if let Some(color) = self.color {
            prof.color = color;
        }

        if let Some(dpi) = self.dpi {
            prof.value = DpiValue::Single(dpi)
        }

        conf.fixup_dpi_metadata();
        dev.send_config(&conf)?;
        Ok(())
    }
}

impl Macro {
    fn run(&self, dev: &mut GloriousDevice) -> Result<()> {
        if self.bank > 3 {
            return Err(anyhow!(
                r"Only 2 macro banks are supported for now,
                TODO find out how many the hardware supports without bricking it"
            ));
        }
        dev.send_macro_bank(self.bank, &self.events)
    }
}

impl Rgb {
    fn run(&self, dev: &mut GloriousDevice) -> Result<()> {
        let mut conf = dev.read_config()?;
        match self {
            Rgb::Off => {
                conf.rgb_current_effect = Effect::Off;
            }
            Rgb::Glorious { direction, speed } => {
                conf.rgb_current_effect = Effect::Glorious;
                if let Some(dir) = direction {
                    conf.rgb_effect_parameters.glorious.direction = dir.into();
                }
                if let Some(spd) = speed {
                    conf.rgb_effect_parameters.glorious.speed = spd.into();
                }
            }
            Rgb::Single { color, brightness } => {
                conf.rgb_current_effect = Effect::SingleColor;
                if let Some(clr) = color {
                    conf.rgb_effect_parameters.single_color.color = *clr;
                }
                if let Some(br) = brightness {
                    conf.rgb_effect_parameters.single_color.brightness = br.into();
                }
            }
            Rgb::Breathing { speed, colors } => {
                conf.rgb_current_effect = Effect::Breathing;
                if let Some(spd) = speed {
                    conf.rgb_effect_parameters.breathing.speed = spd.into();
                }
                if colors.len() > 7 {
                    return Err(anyhow::Error::msg("At most 7 colors are supported."));
                }
                if colors.len() > 0 {
                    conf.rgb_effect_parameters.breathing.count = colors.len().try_into()?;
                    for (i, c) in colors.iter().enumerate() {
                        conf.rgb_effect_parameters.breathing.colors[i] = *c;
                    }
                }
            }
            Rgb::Tail { speed, brightness } => {
                conf.rgb_current_effect = Effect::Tail;
                if let Some(spd) = speed {
                    conf.rgb_effect_parameters.tail.speed = spd.into();
                }
                if let Some(br) = brightness {
                    conf.rgb_effect_parameters.tail.brightness = br.into();
                }
            }
            Rgb::SeamlessBreathing { speed } => {
                conf.rgb_current_effect = Effect::SeamlessBreathing;
                if let Some(spd) = speed {
                    conf.rgb_effect_parameters.seamless_breathing.speed = spd.into();
                }
            }
            Rgb::ConstantRgb { colors } => {
                conf.rgb_current_effect = Effect::ConstantRgb;
                assert!(colors.len() <= 6);
                for (i, c) in colors.iter().enumerate() {
                    conf.rgb_effect_parameters.constant_rgb.colors[i] = *c;
                }
            }
            Rgb::Rave {
                brightness,
                speed,
                colors,
            } => {
                conf.rgb_current_effect = Effect::Rave;
                if let Some(br) = brightness {
                    conf.rgb_effect_parameters.rave.brightness = br.into();
                }
                if let Some(spd) = speed {
                    conf.rgb_effect_parameters.rave.speed = spd.into();
                }
                assert!(colors.len() <= 2);
                for (i, c) in colors.iter().enumerate() {
                    conf.rgb_effect_parameters.rave.colors[i] = *c;
                }
            }
            Rgb::Random { speed } => {
                conf.rgb_current_effect = Effect::Random;
                // HACK: this effect is not available officialy, and it is not properly
                // intialized, with the speed set to 0 (which is likely not a valid value,
                // as it behaves the same as if 0 is set for the speed of other effects,
                // that is the effect is extremely fast).
                // Initialize the value if needed.
                if conf.rgb_effect_parameters.random.speed == 0 {
                    conf.rgb_effect_parameters.random.speed = 1;
                }
                if let Some(spd) = speed {
                    conf.rgb_effect_parameters.random.speed = spd.into();
                }
            }
            Rgb::Wave { brightness, speed } => {
                conf.rgb_current_effect = Effect::Wave;
                if let Some(br) = brightness {
                    conf.rgb_effect_parameters.wave.brightness = br.into();
                }
                if let Some(spd) = speed {
                    conf.rgb_effect_parameters.wave.speed = spd.into();
                }
            }
            Rgb::SingleBreathing { speed, color } => {
                conf.rgb_current_effect = Effect::SingleBreathing;
                if let Some(spd) = speed {
                    conf.rgb_effect_parameters.single_breathing.speed = spd.into();
                }
                if let Some(clr) = color {
                    conf.rgb_effect_parameters.single_breathing.color = *clr;
                }
            }
        };
        dev.send_config(&conf)
    }
}

fn main() -> Result<()> {
    //Dump {}.run()?;
    let opts = Opts::parse();

    let hid = hidapi::HidApi::new()?;
    let mut dev = GloriousDevice::open_first(&hid)?;
    dev.send_msg(0x02, 1)?;

    match opts.cmd {
        Command::Dump(dump) => dump.run(&mut dev),
        Command::Button(b) => b.run(&mut dev),
        Command::Rgb { rgbcmd } => rgbcmd.run(&mut dev),
        Command::Dpi(dpi) => dpi.run(&mut dev),
        Command::Macro(macro_) => macro_.run(&mut dev),
    }
}
