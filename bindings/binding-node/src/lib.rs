use napi::{Result, Error, Status};
use napi_derive::napi;
use flashpoint_archive::Flashpoint;

#[napi(js_name = "Flashpoint")]
pub struct FlashpointNode {
    flashpoint: Flashpoint
}

#[napi]
impl FlashpointNode {
    #[napi(constructor)]
    pub fn new() -> Self {
        FlashpointNode {
            flashpoint: Flashpoint::new()
        }
    }

    #[napi]
    pub fn load_database(&self, source: String) -> Result<()> {
        self.flashpoint.load_database(source.as_str()).map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub fn get_total(&self, table_name: String) -> Result<i64> {
        self.flashpoint.get_total(table_name.as_str()).map_err(|e| {
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
