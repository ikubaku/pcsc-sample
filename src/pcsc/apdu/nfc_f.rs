pub const SYSTEM_ALL: u16 = 0xFFFF;

// 追加で要求するデータ
pub enum RequestCode {
    None,
    SystemCode,
    Capability,
}

impl RequestCode {
    pub fn encode(&self) -> u8 {
        match self {
            RequestCode::None => 0x00,
            RequestCode::SystemCode => 0x01,
            RequestCode::Capability => 0x02,
        }
    }
}

// タイムスロット
pub enum TimeSlot {
    Slot1,
    Slot2,
    Slot4,
    Slot8,
    Slot16,
}

// タイムスロットに指定できる数値は今のところこれだけ
impl TimeSlot {
    pub fn encode(&self) -> u8 {
        match self {
            TimeSlot::Slot1 => 0x00,
            TimeSlot::Slot2 => 0x01,
            TimeSlot::Slot4 => 0x03,
            TimeSlot::Slot8 => 0x07,
            TimeSlot::Slot16 => 0x0F,
        }
    }
}

pub fn polling(
    apdu_buf: &mut [u8; 13],
    system_code: u16,
    request_code: RequestCode,
    time_slot: TimeSlot,
) {
    // ビッグエンディアンに並べかえ
    let system_code_encoded = system_code.to_be_bytes();

    // APDU構築
    apdu_buf[..8].copy_from_slice(b"\xFF\xC2\x00\x01\x08\x95\x06\x06");
    apdu_buf[8] = 0x00;
    apdu_buf[9] = system_code_encoded[0];
    apdu_buf[10] = system_code_encoded[1];
    apdu_buf[11] = request_code.encode();
    apdu_buf[12] = time_slot.encode();
}
