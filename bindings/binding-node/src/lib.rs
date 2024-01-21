use napi::{Result, Error, Status};
use napi_derive::napi;
use flashpoint_archive::{FlashpointArchive, game::{search::{GameSearch, PageTuple}, Game, PartialGame, AdditionalApp}, tag::{Tag, PartialTag}, tag_category::{TagCategory, PartialTagCategory}, game_data::{GameData, PartialGameData}};

#[napi(js_name = "FlashpointArchive")]
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
    pub fn load_database(&mut self, source: String) -> Result<()> {
        self.flashpoint.load_database(source.as_str()).map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn search_games(&self, search: GameSearch) -> Result<Vec<Game>> {
        self.flashpoint.search_games(&search).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn search_games_index(&self, mut search: GameSearch) -> Result<Vec<PageTuple>> {
        self.flashpoint.search_games_index(&mut search).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn search_games_total(&self, search: GameSearch) -> Result<i64> {
        self.flashpoint.search_games_total(&search).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn search_games_with_tag(&self, tag: String) -> Result<Vec<Game>> {
        self.flashpoint.search_games_with_tag(&tag).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn search_games_random(&self, search: GameSearch, count: i64) -> Result<Vec<Game>> {
        self.flashpoint.search_games_random(&search, count).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_game(&self, id: String) -> Result<Option<Game>> {
        self.flashpoint.find_game(&id).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }
    
    #[napi]
    pub async fn create_game(&self, partial_game: PartialGame) -> Result<Game> {
        self.flashpoint.create_game(&partial_game).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn save_game(&self, mut partial_game: PartialGame) -> Result<Game> {
        self.flashpoint.save_game(&mut partial_game).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn save_games(&self, partial_games: Vec<PartialGame>) -> Result<Vec<Game>> {
        let mut saved_games = vec![];
        for mut game in partial_games {
            saved_games.push(self.flashpoint.save_game(&mut game).await.map_err(|e| {
                Error::new(Status::GenericFailure, e)
            })?);
        }
        Ok(saved_games)
    }

    #[napi]
    pub async fn delete_game(&self, id: String) -> Result<()> {
        self.flashpoint.delete_game(&id).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn count_games(&self) -> Result<i64> {
        self.flashpoint.count_games().await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_add_app_by_id(&self, id: String) -> Result<Option<AdditionalApp>> {
        self.flashpoint.find_add_app_by_id(&id).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_all_tags(&self) -> Result<Vec<Tag>> {
        self.flashpoint.find_all_tags().await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_tag(&self, name: String) -> Result<Option<Tag>> {
        self.flashpoint.find_tag(&name).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_tag_by_id(&self, id: i64) -> Result<Option<Tag>> {
        self.flashpoint.find_tag_by_id(id).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn create_tag(&self, name: String, category: Option<String>) -> Result<Tag> {
        self.flashpoint.create_tag(&name, category).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn save_tag(&self, partial: PartialTag) -> Result<Tag> {
        self.flashpoint.save_tag(&partial).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn delete_tag(&self, name: String) -> Result<()> {
        self.flashpoint.delete_tag(&name).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn count_tags(&self) -> Result<i64> {
        self.flashpoint.count_tags().await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn merge_tags(&self, name: String, merged_into: String) -> Result<Tag> {
        self.flashpoint.merge_tags(&name, &merged_into).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_all_platforms(&self) -> Result<Vec<Tag>> {
        self.flashpoint.find_all_platforms().await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_platform(&self, name: String) -> Result<Option<Tag>> {
        self.flashpoint.find_platform(&name).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_platform_by_id(&self, id: i64) -> Result<Option<Tag>> {
        self.flashpoint.find_platform_by_id(id).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn create_platform(&self, name: String) -> Result<Tag> {
        self.flashpoint.create_platform(&name).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn delete_platform(&self, name: String) -> Result<()> {
        self.flashpoint.delete_platform(&name).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn count_platforms(&self) -> Result<i64> {
        self.flashpoint.count_platforms().await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_all_tag_categories(&self) -> Result<Vec<TagCategory>> {
        self.flashpoint.find_all_tag_categories().await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_tag_category(&self, name: String) -> Result<Option<TagCategory>> {
        self.flashpoint.find_tag_category(&name).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_tag_category_by_id(&self, id: i64) -> Result<Option<TagCategory>> {
        self.flashpoint.find_tag_category_by_id(id).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn create_tag_category(&self, partial: PartialTagCategory) -> Result<TagCategory> {
        self.flashpoint.create_tag_category(&partial).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn save_tag_category(&self, partial: PartialTagCategory) -> Result<TagCategory> {
        self.flashpoint.save_tag_category(&partial).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_game_data_by_id(&self, game_data_id: i64) -> Result<Option<GameData>> {
        self.flashpoint.find_game_data_by_id(game_data_id).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_game_data(&self, game_id: String) -> Result<Vec<GameData>> {
        self.flashpoint.find_game_data(&game_id).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn create_game_data(&self, game_data: PartialGameData) -> Result<GameData> {
        self.flashpoint.create_game_data(&game_data).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn save_game_data(&self, game_data: PartialGameData) -> Result<GameData> {
        self.flashpoint.save_game_data(&game_data).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn delete_game_data(&self, id: i64) -> Result<()> {
        self.flashpoint.delete_game_data(id).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn new_tag_filter_index(&self, mut search: GameSearch) -> Result<()> {
        self.flashpoint.new_tag_filter_index(&mut search).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn find_all_game_libraries(&self) -> Result<Vec<String>> {
        self.flashpoint.find_all_game_libraries().await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn add_game_playtime(&self, id: String, seconds: i64) -> Result<()> {
        self.flashpoint.add_game_playtime(&id, seconds).await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn clear_playtime_tracking(&self) -> Result<()> {
        self.flashpoint.clear_playtime_tracking().await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }

    #[napi]
    pub async fn optimize_database(&self) -> Result<()> {
        self.flashpoint.optimize_database().await.map_err(|e| {
            Error::new(Status::GenericFailure, e)
        })
    }
}

#[napi]
pub fn parse_user_search_input(input: String) -> GameSearch {
    flashpoint_archive::game::search::parse_user_input(&input)
}
