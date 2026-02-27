use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::models::{
    ApiGame, CreatePvpRequest, CreateSoloRequest, JoinPvpRequest, PlayMoveRequest,
};

// Small API client wrapper around reqwest.
// In TS terms: this is a service class for HTTP endpoints.
pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn create_solo_game(&self, player_id: &str) -> Result<ApiGame> {
        let url = format!("{}/games/solo", self.base_url);
        let payload = CreateSoloRequest {
            player_id: player_id.to_string(),
            client_name: "rust-tui-client".to_string(),
        };

        let response = self.client.post(url).json(&payload).send().await?;
        parse_json_response(response).await
    }

    pub async fn create_pvp_game(
        &self,
        player_id: &str,
        name: &str,
        password: Option<String>,
    ) -> Result<ApiGame> {
        let url = format!("{}/games/pvp", self.base_url);
        let payload = CreatePvpRequest {
            player_id: player_id.to_string(),
            name: name.to_string(),
            password,
        };

        let response = self.client.post(url).json(&payload).send().await?;
        parse_json_response(response).await
    }

    pub async fn list_open_pvp_games(&self) -> Result<Vec<ApiGame>> {
        let url = format!("{}/games/pvp/open", self.base_url);
        let response = self.client.get(url).send().await?;
        parse_json_response(response).await
    }

    pub async fn join_pvp_game(
        &self,
        player_id: &str,
        game_id: &str,
        password: Option<String>,
    ) -> Result<ApiGame> {
        let url = format!("{}/games/pvp/{game_id}/join", self.base_url);
        let payload = JoinPvpRequest {
            player_id: player_id.to_string(),
            password,
        };

        let response = self.client.post(url).json(&payload).send().await?;
        parse_json_response(response).await
    }

    pub async fn get_game(&self, game_id: &str) -> Result<ApiGame> {
        let url = format!("{}/games/{game_id}", self.base_url);
        let response = self.client.get(url).send().await?;
        parse_json_response(response).await
    }

    pub async fn play_move(&self, player_id: &str, game_id: &str, index: usize) -> Result<ApiGame> {
        let url = format!("{}/games/{game_id}/move", self.base_url);
        let payload = PlayMoveRequest {
            player_id: player_id.to_string(),
            index,
        };

        let response = self.client.post(url).json(&payload).send().await?;
        parse_json_response(response).await
    }
}

async fn parse_json_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T> {
    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());
        anyhow::bail!("request failed with {status}: {body}");
    }

    response
        .json::<T>()
        .await
        .context("invalid JSON response shape")
}
