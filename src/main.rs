use std::{
    sync::{Arc, Mutex, RwLock},
    thread::JoinHandle,
};
mod services;
mod utils;
use services::google::GoogleTranslator;
use services::microsoft::MicrosoftTranslator;

use cmdpal::prelude::*;
use windows::Win32::Foundation::{E_FAIL, ERROR_LOCK_VIOLATION};

use crate::{
    services::Translator,
    utils::{map_fail_err, map_lock_err},
};

#[derive(Clone)]
struct AggTranslator {
    google: Arc<RwLock<GoogleTranslator>>,
    microsoft: Arc<RwLock<MicrosoftTranslator>>,
}

impl AggTranslator {
    fn new() -> Self {
        Self {
            google: Arc::new(RwLock::new(GoogleTranslator)),
            microsoft: Arc::new(RwLock::new(MicrosoftTranslator::new())),
        }
    }

    fn translate_task<T>(
        handle: Arc<RwLock<T>>,
        query: String,
        to_lang: String,
    ) -> JoinHandle<WinResult<String>>
    where
        T: Translator + Sync + Send + 'static,
    {
        std::thread::spawn(move || {
            let read_guard = handle.read().map_err(map_lock_err)?;
            let need_auth = read_guard.auth_required();
            drop(read_guard);
            if need_auth {
                let mut write_guard = handle.write().map_err(map_lock_err)?;
                write_guard.auth().map_err(map_fail_err)?;
                drop(write_guard);
            }
            let read_guard = handle.read().map_err(map_lock_err)?;
            read_guard.translate(&query, &to_lang).map_err(map_fail_err)
        })
    }

    fn translate(&self, query: String, to_lang: String) -> Vec<(String, WinResult<String>)> {
        let google_handle =
            Self::translate_task(self.google.clone(), query.clone(), to_lang.clone());
        let microsoft_handle =
            Self::translate_task(self.microsoft.clone(), query.clone(), to_lang.clone());
        let fetch = |handle: JoinHandle<_>| handle.join().map_err(|_| WinError::from(E_FAIL))?;
        let google_result = fetch(google_handle);
        let microsoft_result = fetch(microsoft_handle);
        vec![
            ("Google".into(), google_result),
            ("Microsoft".into(), microsoft_result),
        ]
    }
}

fn translate_entry_from_result(
    name: String,
    result: WinResult<String>,
) -> WinResult<ComObject<ListItem>> {
    let subtitle = match result.as_ref() {
        Ok(_) => format!("{} Translate, click to copy", &name),
        Err(_) => format!("{} Translate failed", &name),
    };
    Ok(ListItemBuilder::new(
        CommandItemBuilder::try_new(match result {
            Ok(ref s) => CopyTextCommandBuilder::new(s.into()).build().to_interface(),
            Err(_) => NoOpCommandBuilder::new().build().to_interface(),
        })?
        .title(result.unwrap_or_else(|e| e.to_string()))
        .subtitle(subtitle)
        .build(),
    )
    .build())
}

fn main() -> WinResult<()> {
    let list_page = ListPageBuilder::new(
        BasePageBuilder::new(
            BaseCommandBuilder::new()
                .id("BlueG.CmdPal.Translate")
                .name("Machine Translate")
                .icon(IconInfo::from(IconData::from("\u{E82D}")).into())
                .build(),
        )
        .loading(false)
        .title("Machine Translate")
        .build(),
    )
    .placeholder("Type to translate")
    .build();

    let last_request_time = Arc::new(Mutex::new(std::time::Instant::now()));

    let agg_translator = AggTranslator::new();

    let dyn_page = DynamicListPage::new(
        list_page,
        Box::new(move |page, _, new| {
            println!("Search text updated: {}", new.to_string_lossy());
            let query_reach_time = std::time::Instant::now();
            let last_request_time = Arc::clone(&last_request_time);
            let agg_translator = agg_translator.clone();
            let page = page.to_object();
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
                let mut loading_guard = page.loading_mut().ok()?;
                *loading_guard = true;
                drop(loading_guard);

                let results = agg_translator
                    .clone()
                    .translate(new.to_string_lossy(), "zh-Hans".into());
                println!("Translation result: {:?}", &results);
                let last_request_time = last_request_time.clone();
                let mut guard = page.items_mut().ok()?;
                let mut time_guard = last_request_time.lock().ok()?;
                if query_reach_time > *time_guard {
                    *time_guard = query_reach_time;
                    let mut v = vec![];
                    for list_item in results
                        .into_iter()
                        .map(|(name, result)| translate_entry_from_result(name, result))
                    {
                        v.push(list_item.ok()?);
                    }
                    *guard = v;
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
                .title("Translate")
                .subtitle("Translate text with various platforms")
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
