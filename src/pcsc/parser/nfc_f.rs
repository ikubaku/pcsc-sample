use anyhow::{bail, Result};
use nom::bytes::complete::{tag, take};
use nom::combinator::opt;
use nom::IResult;

pub const IDM_LENGTH: usize = 8;
pub const PMM_LENGTH: usize = 8;

pub struct PollingResponse<'data> {
    pub(crate) idm: &'data [u8],
    pub(crate) pmm: &'data [u8],
    pub(crate) requested_data: Option<&'data [u8]>,
}

impl<'data> PollingResponse<'data> {
    pub fn parse_from_data(data: &'data [u8]) -> Result<Self> {
        if let Ok((_, resp)) = Self::parse_polling_response(data) {
            Ok(resp)
        } else {
            bail!("Could not parse the polling response.");
        }
    }

    fn parse_polling_response(data: &'data [u8]) -> IResult<&[u8], Self> {
        // takeは指定したバイト数の任意の数値を受理する
        // tagは指定したバイト列を受理する
        // optは指定したパース内容を0個以上1個以下受理する
        // inputが残りの入力を指すスライス、idmなどその次が受理した数値の位置を指すスライス
        let (input, _) = take(1usize)(data)?;    // フレーム長さ
        let (input, _) = tag(b"\x01")(input)?;    // Pollingレスポンスコード
        let (input, idm) = take(IDM_LENGTH)(input)?;    // IDm
        let (input, pmm) = take(PMM_LENGTH)(input)?;    // PMm
        let (input, requested_data) = opt(take(2usize))(input)?;    // リクエストデータ

        Ok((
            input,
            Self {
                idm,
                pmm,
                requested_data,
            },
        ))
    }
}
