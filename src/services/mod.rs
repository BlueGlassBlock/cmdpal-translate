pub mod google;
pub mod microsoft;

pub trait Translator {
    fn auth_required(&self) -> bool {
        false
    }
    fn auth(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn translate(&self, query: &str, to_lang: &str) -> Result<String, Box<dyn std::error::Error>>;
}
