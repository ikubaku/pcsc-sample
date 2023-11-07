// エントリーポイント
use std::time::Duration;

use anyhow::Result;
use tokio::time::sleep;
use tracing::{error, info, Level};
use tracing_subscriber::{FmtSubscriber, Layer};

use crate::pcsc::ReaderSession;

mod pcsc;

async fn app_main() -> Result<()> {
    // tokio tracingの初期化など
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting sample...");

    // IFDへのダイレクト接続を確立する
    // reader_name引数にはpcsc_scanで出てくるカードリーダーの名前をセットします。
    // 名前の最後についている数値はカードリーダーやスマートカードが他に繋がっていると変わるので無視します。
    // 例: Reader 1: SONY FeliCa RC-S300/P (0201504) 01 00 -> "SONY FeliCa RC-S300/P (0201504)"
    let mut reader_session = ReaderSession::start_session_for_reader("SONY FeliCa RC-S300/P (0201504)")?;
    // コントロールコードの取得
    reader_session.acquire_escape_code_from_reader()?;
    // Transparent Session開始
    reader_session.start_transparent_session()?;
    // NFC-Fにプロトコル変更
    reader_session.switch_protocol_to_nfc_f()?;

    // 1秒ごとにPollingコマンドを発行
    loop {
        sleep(Duration::from_millis(
            1000u64,
        ))
        .await;
        // NFC-F Pollingコマンドを発行
        if reader_session.nfc_f_polling()? {
            println!("IDm: {:?}", reader_session.idm);
        }
    }
    // Transparent Sessionの終了はReaderSessionがdrop（所有されなくなる）されるタイミングで行われる（pcsc.rs 26行目参照）

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    app_main().await.map_err(|e| {
        error!("Fatal: {:?}", e);
        e
    })
}
