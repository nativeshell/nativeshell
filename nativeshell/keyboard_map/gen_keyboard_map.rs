use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct PhysicalKeyEntry {
    names: HashMap<String, String>,
    scan_codes: HashMap<String, serde_json::Value>, // number or array
    key_codes: Option<HashMap<String, serde_json::Value>>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct LogicalKeyEntry {
    name: String,
    value: i64,
    key_label: Option<String>,
    names: HashMap<String, Vec<String>>,
    values: Option<HashMap<String, Vec<i64>>>,
}

#[derive(Debug)]
struct KeyData {
    name: String,
    platform: i64,
    physical: i64,
    logical: Option<i64>,
    fallback: Option<i64>,
}

fn first_number(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Number(n) => n.as_i64(),
        serde_json::Value::Array(a) => a[0].as_i64(),
        _ => None,
    }
}

pub fn generate_keyboard_map(platform_name: &str) -> anyhow::Result<()> {
    let root_dir: PathBuf = std::env::var("CARGO_MANIFEST_DIR")?.into();
    let out_dir: PathBuf = std::env::var("OUT_DIR")?.into();

    let codes_dir = root_dir.join("keyboard_map");
    let physical = fs::read_to_string(codes_dir.join("physical_key_data.json"))?;
    let logical = fs::read_to_string(codes_dir.join("logical_key_data.json"))?;

    let physical_keys: HashMap<String, PhysicalKeyEntry> = serde_json::from_str(&physical)?;
    let logical_keys: HashMap<String, LogicalKeyEntry> = serde_json::from_str(&logical)?;

    let mut key_data = BTreeMap::<i64, KeyData>::new();

    let logical_platform_name: &str = match platform_name {
        "linux" => "gtk",
        name => name,
    };

    let physical_platform_name: &str = match platform_name {
        "linux" => "xkb",
        name => name,
    };

    for v in physical_keys.values() {
        if let (Some(platform), Some(usb)) = (
            v.scan_codes
                .get(physical_platform_name)
                .and_then(first_number),
            v.scan_codes.get("usb").and_then(first_number),
        ) {
            let name = v.names.get("name").unwrap();
            let mut logical = None::<i64>;
            let mut fallback = None::<i64>;
            if let Some(logical_key) = logical_keys.get(name) {
                fallback = Some(logical_key.value);
                if let Some(values) = &logical_key.values {
                    if let Some(values) = values.get(logical_platform_name) {
                        if !values.is_empty() {
                            logical = Some(logical_key.value);
                        }
                    }
                }
            }

            key_data.insert(
                platform,
                KeyData {
                    name: name.into(),
                    platform,
                    physical: usb,
                    logical,
                    fallback, // US layout fallback
                },
            );
        }
    }

    let gen_path = out_dir.join("generated_keyboard_map.rs");
    let mut file = File::create(gen_path)?;
    writeln!(file, "#[allow(dead_code)]")?;
    writeln!(
        file,
        "struct KeyMapEntry {{ platform: i64, physical: i64, logical: Option<i64>, fallback: Option<i64> }}"
    )?;
    writeln!(file)?;
    writeln!(file, "fn get_key_map() -> Vec<KeyMapEntry> {{")?;
    writeln!(file, "    vec![")?;
    for v in key_data.values() {
        writeln!(
            file,
            "        KeyMapEntry {{ platform: {}, physical: {}, logical: {:?}, fallback: {:?} }}, // {}",
            v.platform, v.physical, v.logical, v.fallback, v.name,
        )?;
    }
    writeln!(file, "    ]")?;
    writeln!(file, "}}")?;

    Ok(())
}
