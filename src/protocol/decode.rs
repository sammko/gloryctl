use std::convert::TryFrom;
use std::convert::TryInto;
use std::iter::FromIterator;
use std::str;

use arrayvec::ArrayVec;
use nom::number::complete::be_u8;
use nom::{
    bits, count, do_parse, map, named, pair, switch, tag, take, take_bits, take_str, try_parse,
    tuple, value, IResult,
};

use crate::device;
use crate::device::buttonmap::{ButtonAction, DpiSwitch, MacroMode};
use crate::device::{macros, rgb, Color, Config, DpiProfile, DpiValue, PollingRate};

named!(pub version<&[u8], &str>,
    do_parse!(
        tag!([5, 1]) >>
        ver: take_str!(4usize) >>
        (ver)
    )
);

named!(
    nibble_pair<(u8, u8)>,
    bits!(pair!(take_bits!(4usize), take_bits!(4usize)))
);

named!(take_nibble<(&[u8], usize), u8>, take_bits!(4u8));

fn color_rgb(input: &[u8]) -> IResult<&[u8], Color> {
    let (input, (r, g, b)) = tuple!(input, be_u8, be_u8, be_u8)?;
    Ok((input, Color { r, g, b }))
}

fn color_rbg(input: &[u8]) -> IResult<&[u8], Color> {
    let (input, (r, b, g)) = tuple!(input, be_u8, be_u8, be_u8)?;
    Ok((input, Color { r, g, b }))
}

fn polling_rate((input, offset): (&[u8], usize)) -> IResult<(&[u8], usize), PollingRate> {
    let ((input, offset), pr) = take_nibble((input, offset))?;
    match PollingRate::try_from(pr) {
        Ok(p) => Ok(((input, offset), p)),
        Err(_) => Err(nom::Err::Error(nom::error::Error::new(
            (input, offset),
            nom::error::ErrorKind::Switch,
        ))),
    }
}

fn dpi_profiles_from_raw(
    indep: bool,
    mask: u8,
    values: &[u8],
    colors: &Vec<Color>,
) -> ArrayVec<[DpiProfile; 8]> {
    (0..8)
        .map(|i| DpiProfile {
            color: colors[i],
            enabled: (mask & (1 << (i as u8))) == 0,
            value: if indep {
                DpiValue::Double(dpi_decode(values[2 * i]), dpi_decode(values[2 * i + 1]))
            } else {
                DpiValue::Single(dpi_decode(values[i]))
            },
        })
        .collect::<ArrayVec<_>>()
}

impl rgb::Effect {
    fn parse(input: &[u8]) -> IResult<&[u8], rgb::Effect> {
        let (input, re) = be_u8(input)?;
        match rgb::Effect::try_from(re) {
            Ok(p) => Ok((input, p)),
            Err(_) => Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Switch,
            ))),
        }
    }
}

impl rgb::params::Glorious {
    #[rustfmt::skip]
    named!(
        parse<Self>,
        do_parse!(
            bs: be_u8 >>
            dir: be_u8 >>
            (Self {
                speed: rgb::Speed::from(bs & 0xf),
                direction: rgb::Direction::from(dir)
            })
        )
    );
}

impl rgb::params::SingleColor {
    #[rustfmt::skip]
    named!(
        parse<Self>,
        do_parse!(
            bs: be_u8 >>
            color: color_rbg >>
            (Self {
                brightness: rgb::Brightness::from(bs >> 4),
                color: color
            })
        )
    );
}

impl rgb::params::Breathing {
    #[rustfmt::skip]
    named!(
        parse<Self>,
        do_parse!(
            bs: be_u8 >>
            count: be_u8 >>
            colors: count!(color_rbg, 7) >>
            (Self {
                speed: rgb::Speed::from(bs & 0xf),
                count: count,
                colors: ArrayVec::from_iter(colors)
            })
        )
    );
}

impl rgb::params::Tail {
    #[rustfmt::skip]
    named!(
        parse<Self>,
        do_parse!(
            bs: be_u8 >>
            (Self {
                speed: rgb::Speed::from(bs & 0xf),
                brightness: rgb::Brightness::from(bs >> 4),
            })
        )
    );
}

impl rgb::params::SeamlessBreathing {
    #[rustfmt::skip]
    named!(
        parse<Self>,
        do_parse!(
            bs: be_u8 >>
            (Self {
                speed: rgb::Speed::from(bs & 0xf),
            })
        )
    );
}

impl rgb::params::ConstantRgb {
    #[rustfmt::skip]
    named!(
        parse<Self>,
        do_parse!(
            be_u8 >>
            colors: count!(color_rbg, 6) >>
            (Self {
                colors: ArrayVec::from_iter(colors)
            })
        )
    );
}

impl rgb::params::Rave {
    #[rustfmt::skip]
    named!(
        parse<Self>,
        do_parse!(
            bs: be_u8 >>
            colors: count!(color_rbg, 2) >>
            (Self {
                speed: rgb::Speed::from(bs & 0xf),
                brightness: rgb::Brightness::from(bs >> 4),
                colors: ArrayVec::from_iter(colors)
            })
        )
    );
}

impl rgb::params::Random {
    #[rustfmt::skip]
    named!(
        parse<Self>,
        do_parse!(
            bs: be_u8 >>
            (Self {
                speed: rgb::Speed::from(bs & 0xf),
            })
        )
    );
}

impl rgb::params::Wave {
    #[rustfmt::skip]
    named!(
        parse<Self>,
        do_parse!(
            bs: be_u8 >>
            (Self {
                speed: rgb::Speed::from(bs & 0xf),
                brightness: rgb::Brightness::from(bs >> 4),
            })
        )
    );
}

impl rgb::params::SingleBreathing {
    #[rustfmt::skip]
    named!(
        parse<Self>,
        do_parse!(
            bs: be_u8 >>
            color: color_rbg >>
            (Self {
                speed: rgb::Speed::from(bs & 0xf),
                color: color
            })
        )
    );
}

pub fn config_report(inp: &[u8]) -> IResult<&[u8], Config> {
    // I tried implementing this using the nom macros, but they were not flexible enough, or at
    // least I didn't understand them enough to do what I needed
    let (inp, header) = take!(inp, 9)?;
    let (inp, sensor_id) = be_u8(inp)?;
    let (inp, (indep, pollrate)) = bits!(inp, pair!(take_nibble, polling_rate))?;
    let indep = indep > 0;
    let (inp, (dpi_current, dpi_count)) = nibble_pair(inp)?;
    let (inp, mask) = be_u8(inp)?;
    let (inp, dpi_values) = take!(inp, 16)?;
    let (inp, dpi_colors) = count!(inp, color_rgb, 8)?;
    let dpi_profiles = dpi_profiles_from_raw(indep, mask, dpi_values, &dpi_colors);
    let (inp, light_effect) = rgb::Effect::parse(inp)?;
    let (inp, glorious_param) = rgb::params::Glorious::parse(inp)?;
    let (inp, single_color_param) = rgb::params::SingleColor::parse(inp)?;
    let (inp, breathing_param) = rgb::params::Breathing::parse(inp)?;
    let (inp, tail_param) = rgb::params::Tail::parse(inp)?;
    let (inp, seamless_breathing_param) = rgb::params::SeamlessBreathing::parse(inp)?;
    let (inp, constant_rgb_param) = rgb::params::ConstantRgb::parse(inp)?;
    let (inp, unk1) = take!(inp, 12)?;
    let (inp, rave_param) = rgb::params::Rave::parse(inp)?;
    let (inp, random_param) = rgb::params::Random::parse(inp)?;
    let (inp, wave_param) = rgb::params::Wave::parse(inp)?;
    let (inp, single_breathing_param) = rgb::params::SingleBreathing::parse(inp)?;
    let (inp, (lod, unk2)) = tuple!(inp, be_u8, be_u8)?;
    Ok((
        inp,
        Config {
            header: header.iter().cloned().collect(),
            sensor_id: sensor_id,
            dpi_axes_independent: indep,
            polling_rate: pollrate,
            dpi_current_profile: dpi_current,
            dpi_profile_count: dpi_count,
            dpi_profiles: dpi_profiles,
            rgb_current_effect: light_effect,
            rgb_effect_parameters: rgb::EffectParameters {
                glorious: glorious_param,
                single_color: single_color_param,
                breathing: breathing_param,
                tail: tail_param,
                seamless_breathing: seamless_breathing_param,
                constant_rgb: constant_rgb_param,
                rave: rave_param,
                random: random_param,
                wave: wave_param,
                single_breathing: single_breathing_param,
            },
            lod: lod,
            unknown: (unk1.iter().cloned().collect(), unk2),
        },
    ))
}

fn try_parse_from_u8<T: TryFrom<u8>>(inp: &[u8]) -> IResult<&[u8], T> {
    let (inp, b) = be_u8(inp)?;
    T::try_from(b)
        .map(|v| (inp, v))
        .map_err(|_| nom::Err::Error(nom::error::Error::new(inp, nom::error::ErrorKind::Switch)))
}

impl device::MediaButton {
    fn parse_3b(inp: &[u8]) -> IResult<&[u8], Self> {
        let (inp, v) = take!(inp, 3)?;
        let val = (v[0] as u32) << 16 | (v[1] as u32) << 8 | (v[2] as u32);
        Self::from_bits(val)
            .map(|v| (inp, v))
            .ok_or(nom::Err::Error(nom::error::Error::new(
                inp,
                nom::error::ErrorKind::Switch,
            )))
    }
}

fn dpi_decode(dpiv: u8) -> u16 {
    ((dpiv as u16) + 1) * 100
}

named!(
    button_action<ButtonAction>,
    switch!(be_u8,
        0x11 => map!(tuple!(try_parse_from_u8, take!(2)), |(btn, _)| ButtonAction::MouseButton(btn))
      | 0x12 => map!(take!(3), |v| ButtonAction::Scroll(i8::from_be_bytes([v[0]])))
      | 0x31 => map!(tuple!(try_parse_from_u8, take!(2)), |(btn, v)| ButtonAction::RepeatButton {
          which: btn,
          interval: v[0],
          count: v[1]
        })
      | 0x41 => do_parse!(
          mode: switch!(be_u8,
              0x01 => value!(DpiSwitch::Up)
            | 0x02 => value!(DpiSwitch::Down)
            | 0x00 => value!(DpiSwitch::Cycle)
          ) >>
          _x: take!(2) >>
          (ButtonAction::DpiSwitch(mode))
        )
      | 0x42 => map!(take!(3), |v| ButtonAction::DpiLock(dpi_decode(v[0])))
      | 0x22 => map!(device::MediaButton::parse_3b, |x| ButtonAction::MediaButton(x))
      | 0x21 => map!(tuple!(try_parse_from_u8, take!(2)), |(m, v)| ButtonAction::KeyboardShortcut {
          modifiers: m,
          key: v[1]
        })
      | 0x50 => map!(take!(3), |_| ButtonAction::Disabled)
      | 0x70 => do_parse!(
            bank: be_u8 >>
            x: switch!(be_u8,
                0x01 => map!(be_u8, |c| MacroMode::Burst(c))
              | 0x02 => map!(be_u8, |_| MacroMode::RepeatUntilAnotherPress)
              | 0x04 => map!(be_u8, |_| MacroMode::RepeatUntilRelease)
            ) >>
            (ButtonAction::Macro(bank, x))
        )
    )
);

pub fn buttonmap(inp: &[u8]) -> IResult<&[u8], device::ButtonMapping> {
    let (inp, _) = take!(inp, 8)?;
    let (inp, r) = try_parse!(inp, count!(button_action, 6));

    Ok((inp, r.try_into().unwrap()))
}

impl macros::Event {
    pub fn parse(inp: &[u8]) -> IResult<&[u8], Self> {
        let (inp, (state, typ, duration)): (&[u8], (u8, u8, u16)) = bits!(
            inp,
            tuple!(take_bits!(1u8), take_bits!(3u8), take_bits!(12u16))
        )?;
        let state_ = match state {
            0 => Ok(macros::State::Down),
            1 => Ok(macros::State::Up),
            _ => Err(nom::Err::Error(nom::error::Error::new(
                inp,
                nom::error::ErrorKind::Switch,
            ))),
        }?;
        //let (inp, keycode) = be_u8(inp)?;
        let (inp, evtype_) = match typ {
            0x01 => try_parse_from_u8(inp).map(|(r, v)| (r, macros::EventType::Mouse(v))),
            0x05 => try_parse_from_u8(inp).map(|(r, v)| (r, macros::EventType::Keyboard(v))),
            0x06 => try_parse_from_u8(inp).map(|(r, v)| (r, macros::EventType::Modifier(v))),
            _ => Err(nom::Err::Error(nom::error::Error::new(
                inp,
                nom::error::ErrorKind::Switch,
            ))),
        }?;
        Ok((
            inp,
            Self {
                state: state_,
                evtype: evtype_,
                duration: duration,
            },
        ))
    }
}

impl macros::Macro {
    named!(
        parse<Self>,
        do_parse!(
            _header: take!(8) >>
            bank_number: be_u8 >>
            _unk1: be_u8 >> // maybe part of bank number
            count: be_u8 >>
            events: count!(macros::Event::parse, count.into()) >>
            (Self {bank_number, events})
        )
    );
}
