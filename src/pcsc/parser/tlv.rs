use anyhow::{anyhow, Result};
use nom::bytes::complete::take;
use nom::multi::many0;
use nom::IResult;

pub struct TLVParser<'data> {
    data: &'data [u8],
    entries: Vec<TLVEntry<'data>>,
}

struct TLVEntry<'data> {
    tag: u8,
    data: &'data [u8],    // 記事ではvalueフィールド
}

// TLV形式の情報をTLVEntryの配列にパースする
impl<'data> TLVParser<'data> {
    pub fn parse_slice(data: &'data [u8]) -> Result<Self> {
        match Self::parse_tlv(data) {
            Ok((_, entries)) => Ok(Self { data, entries }),
            Err(_) => Err(anyhow!("Failed to parse TLV data.")),
        }
    }

    pub fn find_value_by_tag(&self, tag: u8) -> Option<&'data [u8]> {
        self.entries.iter().find(|e| e.tag == tag).map(|e| e.data)
    }

    fn parse_tlv(data: &'data [u8]) -> IResult<&'data [u8], Vec<TLVEntry<'data>>> {
        // 0個以上のparse_tlv_entryが受理するデータを受理
        many0(Self::parse_tlv_entry)(data)
    }

    fn parse_tlv_entry(data: &'data [u8]) -> IResult<&'data [u8], TLVEntry<'data>> {
        // takeは指定したバイト数の任意の数値を受理する
        // inputが残りの入力を指すスライス、tagなどその次が受理した数値の位置を指すスライス
        let (input, tag) = take(1usize)(data)?;
        let (input, length) = take(1usize)(input)?;
        let (input, data) = take(length[0] as usize)(input)?;
        Ok((input, TLVEntry { tag: tag[0], data }))
    }
}
