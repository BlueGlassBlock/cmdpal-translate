fn main() {
    const GUID: u128 = 0x594bae2e_624f_436e_a796_70bd5ffc06f2;
    if let Err(e) = cmdpal_packaging::generate_winmd() {
        println!("cargo::warning={}", e);
    }
    if let Err(e) = cmdpal_packaging::AppxManifestBuilder::new()
        .id("BlueG.CmdPal.Translate")
        .display_name("Translation Extension for Command Palette")
        .publisher_display_name("BlueG")
        .class_u128(GUID, None)
        .executable("cmdpal-translate.exe")
        .build()
        .write_xml()
    {
        println!("cargo::warning={}", e);
    }
}
