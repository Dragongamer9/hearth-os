//! A screen's make, model and serial number, read straight from its EDID (the 128-byte block every screen sends):
//! Settings › Display and wlr-randr show "Samsung Odyssey G5" instead of "Unknown". Done here instead of through
//! libdisplay-info, whose versions in Debian and Fedora don't match what smithay-drm-extras builds against.

/// What an EDID says about its screen.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EdidInfo {
    pub make: String,
    pub model: String,
    pub serial: String,
}

/// The three-letter PNP manufacturer IDs seen most on Hearth's devices, as names.
fn vendor_name(pnp: &str) -> Option<&'static str> {
    Some(match pnp {
        "ACI" | "AUS" => "ASUS",
        "ACR" => "Acer",
        "AOC" => "AOC",
        "APP" => "Apple",
        "AUO" => "AU Optronics",
        "BNQ" => "BenQ",
        "BOE" => "BOE",
        "CMN" => "Chimei Innolux",
        "DEL" => "Dell",
        "ENC" => "Eizo",
        "GSM" => "LG",
        "HPN" | "HWP" => "HP",
        "HSD" => "HannStar",
        "IVM" => "Iiyama",
        "LEN" => "Lenovo",
        "LGD" => "LG Display",
        "MSI" => "MSI",
        "NEC" => "NEC",
        "PHL" => "Philips",
        "SAM" | "SEC" => "Samsung",
        "SDC" => "Samsung Display",
        "SHP" => "Sharp",
        "SNY" => "Sony",
        "TCL" => "TCL",
        "VSC" => "ViewSonic",
        "VLV" => "Valve",
        "GBT" => "Gigabyte",
        "XMI" => "Xiaomi",
        _ => return None,
    })
}

fn descriptor_text(d: &[u8]) -> String {
    d.iter()
        .take_while(|&&b| b != 0x0a && b != 0)
        .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { ' ' })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Reads the first EDID block. None when it isn't one (wrong header or too short).
pub fn parse(edid: &[u8]) -> Option<EdidInfo> {
    if edid.len() < 128 || edid[0..8] != [0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00] {
        return None;
    }
    let id = u16::from_be_bytes([edid[8], edid[9]]);
    let letter = |v: u16| (b'A' + ((v & 0x1f) as u8).saturating_sub(1)) as char;
    let pnp: String = [letter(id >> 10), letter(id >> 5), letter(id)].iter().collect();
    let product = u16::from_le_bytes([edid[10], edid[11]]);
    let serial_num = u32::from_le_bytes([edid[12], edid[13], edid[14], edid[15]]);

    let mut name = String::new();
    let mut serial = String::new();
    for i in 0..4 {
        let d = &edid[54 + i * 18..54 + (i + 1) * 18];
        // A display descriptor (not a timing): 00 00 00 tag 00 then 13 bytes.
        if d[0] != 0 || d[1] != 0 || d[2] != 0 {
            continue;
        }
        match d[3] {
            0xfc => name = descriptor_text(&d[5..]),
            0xff => serial = descriptor_text(&d[5..]),
            _ => {}
        }
    }
    let make = vendor_name(&pnp).map(str::to_string).unwrap_or(pnp);
    let model = if name.is_empty() { format!("0x{product:04X}") } else { name };
    if serial.is_empty() && serial_num != 0 {
        serial = serial_num.to_string();
    }
    Some(EdidInfo { make, model, serial })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up but well-formed EDID: Samsung (SAM), product 0x7067, a monitor name and a serial descriptor.
    fn sample() -> Vec<u8> {
        let mut e = vec![0u8; 128];
        e[0..8].copy_from_slice(&[0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00]);
        // "SAM": S=19, A=1, M=13 -> 0b0_10011_00001_01101
        let v: u16 = (19 << 10) | (1 << 5) | 13;
        e[8..10].copy_from_slice(&v.to_be_bytes());
        e[10..12].copy_from_slice(&0x7067u16.to_le_bytes());
        e[12..16].copy_from_slice(&0x0000_0001u32.to_le_bytes());
        // Descriptor 1: a timing (skipped); 2: name; 3: serial.
        e[54] = 0x01;
        let name = b"Odyssey G5\n  ";
        e[72..77].copy_from_slice(&[0, 0, 0, 0xfc, 0]);
        e[77..77 + 13].copy_from_slice(name);
        let sn = b"H4ZR500123\n  ";
        e[90..95].copy_from_slice(&[0, 0, 0, 0xff, 0]);
        e[95..95 + 13].copy_from_slice(sn);
        e
    }

    #[test]
    fn reads_make_model_and_serial() {
        let i = parse(&sample()).unwrap();
        assert_eq!(i.make, "Samsung");
        assert_eq!(i.model, "Odyssey G5");
        assert_eq!(i.serial, "H4ZR500123");
    }

    #[test]
    fn no_name_falls_back_to_the_product_code_and_bad_blocks_are_none() {
        let mut e = sample();
        for b in &mut e[72..108] {
            *b = 0;
        }
        e[72] = 1; // now timings, not descriptors
        e[90] = 1;
        let i = parse(&e).unwrap();
        assert_eq!(i.model, "0x7067");
        assert_eq!(i.serial, "1");
        assert!(parse(&e[..100]).is_none());
        let mut bad = sample();
        bad[0] = 1;
        assert!(parse(&bad).is_none());
    }
}
