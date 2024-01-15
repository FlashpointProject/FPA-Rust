use napi::{Result, Error, Status};
use napi_derive::napi;
use flashpoint_archive::{FlashpointArchive, game::{search::GameSearch, Game}};

#[napi(js_name = "Flashpoint")]
pub struct FlashpointNode {
    flashpoint: FlashpointArchive
}

#[napi]
impl FlashpointNode {
    #[napi(constructor)]
    pub fn new() -> Self {
        FlashpointNode {
            flashpoint: FlashpointArchive::new()
        }
    }

    #[napi]
    pub fn load_database(&self, source: String) -> Result<()> {
        self.flashpoint.load_database(source.as_str()).map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub fn search_games(&self, search: GameSearch) -> Result<Vec<Game>> {
        self.flashpoint.search_games(&search).map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }
}

#[napi]
pub fn add(left: i32, right: i32) -> i32 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
