use crate::device::macros;
use crate::device::{
    buttonmap::{ButtonAction, DpiSwitch, MacroMode},
    rgb, ButtonMapping, Color, Config, DataReport, DpiValue,
};

struct ByteBuffer {
    buf: Vec<u8>,
}

impl ByteBuffer {
    fn with_capacity(cap: usize) -> Self {
        ByteBuffer {
            buf: Vec::<u8>::with_capacity(cap),
        }
    }

    fn put_byte(&mut self, b: u8) {
        self.buf.push(b);
    }

    fn put_bytes(&mut self, bs: &[u8]) {
        self.buf.extend(bs);
    }

    fn to_raw_config(&self) -> DataReport {
        // TODO: This entire function is trash.

        let mut padded = self.buf.clone();
        assert!(padded.len() <= 520);
        padded.extend(vec![0; 520 - padded.len()]);
        let mut raw: DataReport = [0; 520];
        raw.copy_from_slice(&padded);
        raw
    }
}

impl Color {
    fn put_rgb(&self, out: &mut ByteBuffer) {
        out.put_bytes(&[self.r, self.g, self.b]);
    }

    fn put_rbg(&self, out: &mut ByteBuffer) {
        out.put_bytes(&[self.r, self.b, self.g])
    }
}

impl rgb::params::Glorious {
    fn put(&self, out: &mut ByteBuffer) {
        out.put_byte(self.speed | 0x40); // Default ignored brightness
        out.put_byte(self.direction);
    }
}

impl rgb::params::SingleColor {
    fn put(&self, out: &mut ByteBuffer) {
        out.put_byte(self.brightness << 4);
        self.color.put_rbg(out);
    }
}

impl rgb::params::Breathing {
    fn put(&self, out: &mut ByteBuffer) {
        out.put_byte(self.speed | 0x40); // Default ignored brightness
        out.put_byte(self.count);
        for color in self.colors.iter() {
            color.put_rbg(out);
        }
    }
}

impl rgb::params::Tail {
    fn put(&self, out: &mut ByteBuffer) {
        out.put_byte(self.speed | self.brightness << 4);
    }
}

impl rgb::params::SeamlessBreathing {
    fn put(&self, out: &mut ByteBuffer) {
        out.put_byte(self.speed | 0x40); // Default ignored brightness
    }
}

impl rgb::params::ConstantRgb {
    fn put(&self, out: &mut ByteBuffer) {
        out.put_byte(0); // Default ignored BS
        for color in self.colors.iter() {
            color.put_rbg(out);
        }
    }
}

impl rgb::params::Rave {
    fn put(&self, out: &mut ByteBuffer) {
        out.put_byte(self.speed | self.brightness << 4);
        for color in self.colors.iter() {
            color.put_rbg(out);
        }
    }
}

impl rgb::params::Random {
    fn put(&self, out: &mut ByteBuffer) {
        out.put_byte(self.speed | 0x00); // Default ignored brightness
    }
}

impl rgb::params::Wave {
    fn put(&self, out: &mut ByteBuffer) {
        out.put_byte(self.speed | self.brightness << 4);
    }
}

impl rgb::params::SingleBreathing {
    fn put(&self, out: &mut ByteBuffer) {
        out.put_byte(self.speed | 0x00); // Default ignored brightness
        self.color.put_rbg(out);
    }
}

pub fn config_report(cfg: &Config) -> DataReport {
    let mut buf = ByteBuffer::with_capacity(520);
    buf.put_bytes(&cfg.header);
    buf.put_byte(cfg.sensor_id);
    buf.put_byte(u8::from(cfg.dpi_axes_independent) << 4 | cfg.polling_rate as u8);
    buf.put_byte(cfg.dpi_current_profile << 4 | cfg.dpi_profile_count);
    let mut enabled_mask = 0u8;
    for (i, dpi) in cfg.dpi_profiles.iter().enumerate() {
        enabled_mask |= u8::from(!dpi.enabled) << (i as u8);
    }
    buf.put_byte(enabled_mask);
    for dpi in cfg.dpi_profiles.iter() {
        match dpi.value {
            DpiValue::Single(x) => buf.put_byte(x),
            DpiValue::Double(x, y) => {
                buf.put_byte(x);
                buf.put_byte(y);
            }
        }
    }
    if !cfg.dpi_axes_independent {
        buf.put_bytes(&[0; 8]);
    }
    for dpi in cfg.dpi_profiles.iter() {
        dpi.color.put_rgb(&mut buf);
    }
    buf.put_byte(cfg.rgb_current_effect as u8);
    cfg.rgb_effect_parameters.glorious.put(&mut buf);
    cfg.rgb_effect_parameters.single_color.put(&mut buf);
    cfg.rgb_effect_parameters.breathing.put(&mut buf);
    cfg.rgb_effect_parameters.tail.put(&mut buf);
    cfg.rgb_effect_parameters.seamless_breathing.put(&mut buf);
    cfg.rgb_effect_parameters.constant_rgb.put(&mut buf);
    buf.put_bytes(&cfg.unknown.0);
    cfg.rgb_effect_parameters.rave.put(&mut buf);
    cfg.rgb_effect_parameters.random.put(&mut buf);
    cfg.rgb_effect_parameters.wave.put(&mut buf);
    cfg.rgb_effect_parameters.single_breathing.put(&mut buf);
    buf.put_byte(cfg.lod);
    buf.put_byte(cfg.unknown.1);
    buf.to_raw_config()
}

impl ButtonAction {
    fn put(&self, out: &mut ByteBuffer) {
        match self {
            ButtonAction::MouseButton(b) => out.put_bytes(&[0x11, b.bits(), 0x00, 0x00]),
            ButtonAction::Scroll(b) => out.put_bytes(&[0x12, *b, 0x00, 0x00]),
            ButtonAction::RepeatButton {
                which,
                interval,
                count,
            } => out.put_bytes(&[0x31, *which, *interval, *count]),
            ButtonAction::DpiSwitch(sw) => {
                out.put_byte(0x41);
                match sw {
                    DpiSwitch::Cycle => out.put_byte(0x00),
                    DpiSwitch::Up => out.put_byte(0x01),
                    DpiSwitch::Down => out.put_byte(0x02),
                };
                out.put_bytes(&[0x00, 0x00]);
            }
            ButtonAction::DpiLock(b) => out.put_bytes(&[0x42, *b, 0x00, 0x00]),
            ButtonAction::MediaButton(x) => {
                let bs = x.bits().to_be_bytes();
                out.put_bytes(&[0x22, bs[1], bs[2], bs[3]]);
            }
            ButtonAction::KeyboardShortcut { modifiers, key } => {
                out.put_bytes(&[0x21, modifiers.bits(), *key, 0x00])
            }
            ButtonAction::Disabled => out.put_bytes(&[0x50, 0x01, 0x00, 0x00]),
            ButtonAction::Macro(bank, mode) => {
                out.put_bytes(&[0x70, *bank]);
                match mode {
                    MacroMode::Burst(c) => out.put_bytes(&[0x01, *c]),
                    MacroMode::RepeatUntilRelease => out.put_bytes(&[0x04, 0x01]),
                    MacroMode::RepeatUntilAnotherPress => out.put_bytes(&[0x02, 0x01]),
                }
            }
        }
    }
}

pub fn buttonmap(mapping: &ButtonMapping) -> DataReport {
    let mut buf = ByteBuffer::with_capacity(520);
    buf.put_bytes(&[0x04, 0x12, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00]);
    for m in mapping {
        m.put(&mut buf);
    }
    for _ in mapping.len()..20 {
        ButtonAction::Disabled.put(&mut buf);
    }
    buf.to_raw_config()
}

impl macros::Event {
    #[allow(dead_code)]
    fn put(&self, out: &mut ByteBuffer) {
        let mut b1 = 0u8;
        b1 |= match self.state {
            macros::State::Up => 1 << 7,
            macros::State::Down => 0 << 7,
        };

        let (typ, keycode) = match self.evtype {
            macros::EventType::Keyboard(c) => (5, c),
            macros::EventType::Modifier(c) => (6, c.bits()),
            macros::EventType::Mouse(c) => (1, c.bits()),
        };

        b1 |= typ << 4;
        let duration_bytes = self.duration.to_be_bytes();
        b1 |= duration_bytes[0] & 0xf;
        out.put_bytes(&[b1, duration_bytes[1], keycode]);
    }
}
