// APDU定義
pub mod nfc_f;

// APDUのデータ内容
pub mod transparent {
    pub const START_SESSION: &[u8; 8] = b"\xFF\xC2\x00\x00\x02\x81\x00\x00";
    pub const END_SESSION: &[u8; 8] = b"\xFF\xC2\x00\x00\x02\x82\x00\x00";
    pub const SWITCH_TO_NFC_F: &[u8; 10] = b"\xFF\xC2\x00\x02\x04\x8F\x02\x03\x00\x00";
}
