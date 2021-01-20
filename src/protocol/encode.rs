use crate::device::{rgb, Color, Config, DpiValue, RawConfig};

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

    fn to_raw_config(&self) -> RawConfig {
        // TODO: This entire function is trash.

        let mut padded = self.buf.clone();
        assert!(padded.len() <= 520);
        padded.extend(vec![0; 520 - padded.len()]);
        let mut raw: RawConfig = [0; 520];
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

pub fn config_report(cfg: &Config) -> RawConfig {
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
