use anyhow::Result;

use gloryctl::{ButtonAction, DataReport, GloriousDevice};

#[allow(dead_code)]
fn dump_data_report(r: &DataReport) {
    for x in r.chunks(16) {
        for byte in x {
            print!("{:02x} ", byte);
        }
        print!("\n");
    }
}

fn main() -> Result<()> {
    let hid = hidapi::HidApi::new()?;
    let mut dev = GloriousDevice::open_first(&hid)?;

    //dbg!(dev.read_fw_version()?);

    //dump_data_report(&dev.read_buttonmap_raw()?);
    //dump_data_report(&dev.read_config_raw()?);
    let mut map = dev.read_buttonmap()?;
    //println!("{:?}", &map);
    map[5] = ButtonAction::KeyboardShortcut {
        modifiers: 8,
        key: 0x28,
    };

    dev.send_buttonmap(&map)?;

    // let conf = dev.read_config()?;
    // dbg!(conf);

    // conf.rgb_current_effect = rgb::Effect::Off;
    // conf.rgb_effect_parameters.single_color.brightness = 1;

    // dev.send_config(&conf)?;

    Ok(())
}
