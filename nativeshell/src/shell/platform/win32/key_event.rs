use super::all_bindings::*;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct KeyEvent {
    key_code: u32,
    scan_code: u32,
    characters_ignoring_modifiers_ex: Option<String>,
    characters_ignoring_modifiers_except_shift_ex: Option<String>,
    #[serde(flatten)]
    other: serde_json::Map<String, serde_json::Value>,
}

pub fn process_key_event(data: Vec<u8>) -> Vec<u8> {
    let mut event: KeyEvent = serde_json::from_slice(&data).unwrap();

    let key_state = &mut [0u8; 256];
    let buf = &mut [0u16, 10];
    let buf_shift = &mut [0u16, 10];

    unsafe {
        let res_1 = ToUnicode(
            event.key_code,
            event.scan_code,
            key_state.as_ptr(),
            PWSTR(buf.as_mut_ptr()),
            buf.len() as i32,
            0,
        );

        key_state[VK_SHIFT as usize] = 128;
        let res_2 = ToUnicode(
            event.key_code,
            event.scan_code,
            key_state.as_ptr(),
            PWSTR(buf_shift.as_mut_ptr()),
            buf_shift.len() as i32,
            0,
        );

        if res_1 == 1 {
            event.characters_ignoring_modifiers_ex = Some(String::from_utf16_lossy(&buf[0..1]));
        }
        if res_2 == 1 {
            event.characters_ignoring_modifiers_except_shift_ex =
                Some(String::from_utf16_lossy(&buf_shift[0..1]));
        }
    }

    serde_json::to_vec(&event).unwrap()
}
