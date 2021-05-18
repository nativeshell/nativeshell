use gdk::{keyval_to_unicode, Display, Keymap, ModifierType};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct KeyEvent {
    characters_ignoring_modifiers_ex: Option<String>,
    characters_ignoring_modifiers_except_shift_ex: Option<String>,
    scan_code: isize,

    #[serde(flatten)]
    other: serde_json::Map<String, serde_json::Value>,
}

pub fn process_key_event(event: Vec<u8>) -> Vec<u8> {
    let mut event: KeyEvent = serde_json::from_slice(&event).unwrap();
    if let Some(display) = Display::get_default() {
        if let Some(keymap) = Keymap::get_for_display(&display) {
            if let Some(state) =
                keymap.translate_keyboard_state(event.scan_code as u32, ModifierType::empty(), 0)
            {
                if let Some(unicode) = keyval_to_unicode(state.0) {
                    event.characters_ignoring_modifiers_ex = Some(unicode.to_string());
                }
            }
            if let Some(state) =
                keymap.translate_keyboard_state(event.scan_code as u32, ModifierType::SHIFT_MASK, 0)
            {
                if let Some(unicode) = keyval_to_unicode(state.0) {
                    event.characters_ignoring_modifiers_except_shift_ex = Some(unicode.to_string());
                }
            }
        }
    }

    serde_json::to_vec(&event).unwrap()
}
