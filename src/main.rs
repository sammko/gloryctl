use anyhow::Result;

use gloryctl::GloriousDevice;

fn main() -> Result<()> {
    let hid = hidapi::HidApi::new()?;
    let dev = GloriousDevice::open_first(&hid)?;

    dbg!(dev.read_fw_version()?);

    let conf = dev.read_config()?;
    dbg!(conf);

    // conf.rgb_current_effect = rgb::Effect::Off;
    // conf.rgb_effect_parameters.single_color.brightness = 1;

    // dev.send_config(&conf)?;

    Ok(())
}