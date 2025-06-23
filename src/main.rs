use std::sync::{Arc, Mutex};

use cmdpal::prelude::*;
use windows::Win32::Foundation::{E_INVALIDARG, ERROR_LOCK_VIOLATION, ERROR_NETWORK_UNREACHABLE};

fn request_google_translate(query: &str) -> WinResult<String> {
    let url = reqwest::Url::parse_with_params(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl=auto&tl=zh-Hans&dt=t&strip=1&nonced=1",
        &[("q", query)],
    )
    .map_err(|_| windows::core::Error::new(E_INVALIDARG, "Invalid query string"))?;

    let client = reqwest::blocking::Client::new();
    let response: serde_json::Value = client
        .get(url)
        .send()
        .map_err(|e| {
            windows::core::Error::new(ERROR_NETWORK_UNREACHABLE.to_hresult(), e.to_string())
        })?
        .json()
        .map_err(|e| windows::core::Error::new(E_INVALIDARG, e.to_string()))?;

    response
        .get(0)
        .and_then(|v| v.get(0))
        .and_then(|v| v.get(0))
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| windows::core::Error::new(E_INVALIDARG, "No translation found"))
}

fn main() -> WinResult<()> {
    let list_page = ListPageBuilder::new(
        BasePageBuilder::new(
            BaseCommandBuilder::new()
                .id("BlueG.CmdPal.Translate.Google")
                .name("Google Translate")
                .icon(IconInfo::from(IconData::from("\u{E82D}")).into())
                .build(),
        )
        .title("Google Translate")
        .build(),
    )
    .placeholder("Type to translate")
    .build();

    let last_request_time = Arc::new(Mutex::new(std::time::Instant::now()));

    let dyn_page = DynamicListPage::new(
        list_page,
        Box::new(move |page, _, new| {
            println!("Search text updated: {}", new.to_string_lossy());
            let query_reach_time = std::time::Instant::now();
            let last_request_time = Arc::clone(&last_request_time);
            let page = page.base.to_object();
            if new.is_empty() {
                let mut guard = page.items_mut()?;
                *guard = vec![];
                let mut time_guard = last_request_time
                    .lock()
                    .map_err(|_| windows::core::Error::from(ERROR_LOCK_VIOLATION))?;
                *time_guard = query_reach_time;
                return Ok(());
            }
            std::thread::spawn(move || -> Option<()> {
                let v = request_google_translate(&new.to_string_lossy()).ok()?;
                println!("Translation result: {}", &v);
                let last_request_time = last_request_time.clone();
                let mut guard = page.items_mut().ok()?;
                let mut time_guard = last_request_time.lock().ok()?;
                if query_reach_time > *time_guard {
                    *time_guard = query_reach_time;
                    *guard = vec![
                        ListItemBuilder::new(
                            CommandItemBuilder::try_new(
                                CopyTextCommandBuilder::new(HSTRING::from(&v))
                                    .build()
                                    .to_interface(),
                            )
                            .ok()?
                            .title(v.to_string())
                            .subtitle("Click to copy")
                            .build(),
                        )
                        .build(),
                    ];
                }
                Some(())
            });

            Ok(())
        }),
    );

    let provider = CommandProviderBuilder::new()
        .id("BlueG.CmdPal.Translate")
        .display_name("Translation Extension for Command Palette")
        .frozen(true)
        .icon(IconInfo::from(IconData::from("\u{F2B7}")).into())
        .add_top_level(
            CommandItemBuilder::try_new(dyn_page.to_interface())?
                .title("Google Translate")
                .subtitle("Translate text using Google Translate")
                .icon(IconInfo::from(IconData::from("\u{E82D}")).into())
                .build()
                .to_interface(),
        )
        .build();
    ExtRegistry::new()
        .register(
            GUID::from_u128(0x594bae2e_624f_436e_a796_70bd5ffc06f2),
            Extension::from(&*provider),
        )
        .serve()
}
