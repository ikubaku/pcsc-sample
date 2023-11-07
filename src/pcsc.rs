// PC/SC通信ビジネスモジュール
use std::ffi::CString;

use crate::pcsc::apdu::nfc_f::{RequestCode, TimeSlot};
use crate::pcsc::parser::tlv::TLVParser;
use crate::pcsc::parser::ResponseApduError;
use anyhow::{anyhow, bail, Result};
use pcsc::ffi::DWORD;
use pcsc::{ctl_code, Card, Context, Protocols, Scope, ShareMode, MAX_BUFFER_SIZE};
use tracing::{debug, info, warn};

mod apdu;
mod parser;

const IDM_LENGTH: usize = 8;

pub struct ReaderSession {
    ctx: Context,
    reader_name: CString,
    recv_buf: [u8; MAX_BUFFER_SIZE],    // ここにIFD Handlerから返却されたバイト列が入る
    escape_code: Option<DWORD>,
    direct_reader: Card,
    is_transparent_mode: bool,
    pub(crate) idm: [u8; IDM_LENGTH],
}

impl Drop for ReaderSession {
    // dropをフックする関数。デストラクタのようなもの
    fn drop(&mut self) {
        if self.is_transparent_mode {
            self.end_transparent_session().unwrap();
        }
    }
}

impl ReaderSession {
    pub fn start_session_for_reader(reader_name: &str) -> Result<Self> {
        let ctx = Context::establish(Scope::User)?;
        let reader_name = Self::find_reader(&ctx, reader_name)?;
        // 共有モード: ダイレクトでIFDと直接う通信するセッションを作る
        let direct_reader = ctx.connect(&reader_name, ShareMode::Direct, Protocols::UNDEFINED)?;
        Ok(Self {
            ctx,
            reader_name,
            recv_buf: [0; MAX_BUFFER_SIZE],
            escape_code: None,
            direct_reader,
            is_transparent_mode: false,
            idm: [0; IDM_LENGTH],
        })
    }

    pub fn use_default_escape_code(&mut self) {
        // PC/SC specifications part 3 sup2-v2.02.00 section 3.3
        self.escape_code = Some(ctl_code(3500));
    }

    pub fn acquire_escape_code_from_reader(&mut self) -> Result<()> {
        // SCARD_CTL_CODE(3400)コントロールコードで機能リスト取得。FEATURE_CCID_ESC_COMMANDのコントロールコードを取得する
        self.direct_reader
            .control(ctl_code(3400), b"", &mut self.recv_buf)?;
        let resp_parser = TLVParser::parse_slice(&self.recv_buf)?;
        if let Some(code) = parser::get_escape_code_from_get_feature_request_response(&resp_parser)
        {
            debug!("Acquired CCID escape code: {}", code);
            self.escape_code = Some(code)
        } else {
            warn!("Could not acquire the escape code from the card reader. Using the default escape code.");
            self.use_default_escape_code();
        }

        Ok(())
    }

    pub fn start_transparent_session(&mut self) -> Result<()> {
        if let Some(code) = self.escape_code {
            // Manage Session: Start Transparent Session
            self.direct_reader.control(
                code,
                apdu::transparent::START_SESSION,
                &mut self.recv_buf,
            )?;
        } else {
            bail!("CCID escape code is not available.");
        }
        Ok(())
    }

    pub fn end_transparent_session(&mut self) -> Result<()> {
        if let Some(code) = self.escape_code {
            // Manage Session: End Transparent Session
            self.direct_reader
                .control(code, apdu::transparent::END_SESSION, &mut self.recv_buf)?;
        } else {
            bail!("CCID escape code is not available.");
        }
        Ok(())
    }

    pub fn switch_protocol_to_nfc_f(&mut self) -> Result<()> {
        if let Some(code) = self.escape_code {
            // Switch Protocol: Switch To NFC-F
            self.direct_reader.control(
                code,
                apdu::transparent::SWITCH_TO_NFC_F,
                &mut self.recv_buf,
            )?;
        } else {
            bail!("CCID escape code is not available.");
        }
        Ok(())
    }

    pub fn nfc_f_polling(&mut self) -> Result<bool> {
        if let Some(code) = self.escape_code {
            let mut apdu_buf: [u8; 13] = [0; 13];
            // NFC-F Pollingコマンドを発行するTransparent Exchange: Transceive Command APDUを構築する
            apdu::nfc_f::polling(
                &mut apdu_buf,
                apdu::nfc_f::SYSTEM_ALL,
                RequestCode::None,
                TimeSlot::Slot1,
            );
            // Command APDU送信
            self.direct_reader
                .control(code, &apdu_buf, &mut self.recv_buf)?;
        } else {
            bail!("CCID escape code is not available.");
        }
        let resp_parser = TLVParser::parse_slice(&self.recv_buf)?;
        // まず tag == 0xC1 のTLVを探してエラー状態を確認
        if let Some(error) = parser::get_response_apdu_error(&resp_parser) {
            Ok(match error {
                ResponseApduError::Ok => {
                    // 通信成功したのでNFC-Fタグから応答があったと仮定
                    info!("Found a NFC-F card.");
                    // Responseデータオブジェクトをパース
                    if let Some(idm) = self.acquire_idm(&resp_parser)? {
                        self.idm.copy_from_slice(&idm);
                        true
                    } else {
                        false
                    }
                }
                // とりあえずタグから反応しなかった場合だけフック（正直サボってます）
                ResponseApduError::ICCNotResponding => {
                    info!("No response from ICC.");
                    false
                }
                _ => {
                    warn!("Unexpected response error: {:?}", error);
                    false
                }
            })
        } else {
            Ok(false)
        }
    }

    fn acquire_idm(&self, parser: &TLVParser) -> Result<Option<Vec<u8>>> {
        // Responseデータオブジェクトだけ抜き出す
        if let Some(data) = parser::get_response_data(parser) {
            // Pollingレスポンスフレームをパース
            let polling_resp = parser::nfc_f::PollingResponse::parse_from_data(&data)?;
            Ok(Some(Vec::<u8>::from(polling_resp.idm)))
        } else {
            warn!("No response data object in IFC response.");
            Ok(None)
        }
    }

    // PCSC liteに認識されているカードリーダーを名前で検索して、実際の名前（数値がついている状態の名前）を引いてくる
    fn find_reader(ctx: &Context, reader_name: &str) -> Result<CString> {
        let reader_names = ctx.list_readers_owned()?;
        let reader_name_to_find_bytes = reader_name.as_bytes();
        let found_reader_name = reader_names
            .iter()
            .fold(None, |acc, reader_name| {
                if acc.is_some() {
                    return acc;
                }
                let reader_name_bytes = reader_name.to_bytes();
                if reader_name_bytes.len() < reader_name_to_find_bytes.len() {
                    return None;
                }

                if reader_name_to_find_bytes
                    .iter()
                    .enumerate()
                    .fold(true, |acc, (i, to_find_b)| {
                        acc && (*to_find_b == reader_name_bytes[i])
                    })
                {
                    info!("Found matching reader: {:?}", reader_name);
                    Some(reader_name)
                } else {
                    None
                }
            })
            .ok_or(anyhow!("Reader not found."))?;
        Ok(CString::new(found_reader_name.as_bytes()).unwrap())
    }
}
