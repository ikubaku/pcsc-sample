// 各種データパーサ
use pcsc::ffi::DWORD;
use tracing::warn;

mod constants;
pub mod nfc_f;
pub mod tlv;

use constants::{ifd_features, response, response_data};
use tlv::TLVParser;

#[derive(Debug)]
pub enum ResponseApduError {
    Ok,
    DataNotAvailable,
    NoInformation,
    BadRelatedDataObject,
    NotSupported,
    UnexpectedLength,
    UnexpectedValue,
    IFDNotResponding,
    ICCNotResponding,
    NoPreciseInfo,
    Unknown,
    MalformedApdu,
}

pub fn get_escape_code_from_get_feature_request_response(resp_parser: &TLVParser) -> Option<DWORD> {
    if let Some(code_val) = resp_parser.find_value_by_tag(ifd_features::FEATURE_CCID_ESC_COMMAND) {
        // tag == 0x13のTLVがあれば内容を取得
        if code_val.len() != 4 {
            warn!(
                "Unexpected FEATURE_CCID_ESC_COMMAND length: {}",
                code_val.len()
            );
            None
        } else {
            Some(u32::from_be_bytes(code_val.try_into().unwrap()) as DWORD)
        }
    } else {
        None
    }
}

pub fn get_response_apdu_error(resp_parser: &TLVParser) -> Option<ResponseApduError> {
    if let Some(resp_value) = resp_parser.find_value_by_tag(response::APDU_TAG) {
        if resp_value.len() != 3 {
            warn!(
                "Malformed APDU: wrong response APDU length: {}",
                resp_value.len()
            );
            return Some(ResponseApduError::MalformedApdu);
        }

        let error_code = &resp_value[1..];
        if error_code[0] == 0x90 && error_code[1] == 0x00 {
            Some(ResponseApduError::Ok)
        } else if error_code[0] == 0x62 && error_code[1] == 0x82 {
            Some(ResponseApduError::DataNotAvailable)
        } else if error_code[0] == 0x63 && error_code[1] == 0x00 {
            Some(ResponseApduError::NoInformation)
        } else if error_code[0] == 0x63 && error_code[1] == 0x01 {
            Some(ResponseApduError::BadRelatedDataObject)
        } else if error_code[0] == 0x6A && error_code[1] == 0x81 {
            Some(ResponseApduError::NotSupported)
        } else if error_code[0] == 0x67 && error_code[1] == 0x00 {
            Some(ResponseApduError::UnexpectedLength)
        } else if error_code[0] == 0x6A && error_code[1] == 0x80 {
            Some(ResponseApduError::UnexpectedValue)
        } else if error_code[0] == 0x64 && error_code[1] == 0x00 {
            Some(ResponseApduError::IFDNotResponding)
        } else if error_code[0] == 0x64 && error_code[1] == 0x01 {
            Some(ResponseApduError::ICCNotResponding)
        } else if error_code[0] == 0x6F && error_code[1] == 0x00 {
            Some(ResponseApduError::NoPreciseInfo)
        } else {
            Some(ResponseApduError::Unknown)
        }
    } else {
        None
    }
}

pub fn get_response_data(resp_parser: &TLVParser) -> Option<Vec<u8>> {
    // Responseデータオブジェクトのタグで検索
    resp_parser
        .find_value_by_tag(response_data::APDU_TAG)
        .map(Vec::<u8>::from)
}
