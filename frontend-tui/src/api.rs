// ========================
// Rust API Client Example
// ========================
// This file implements a simple API client in Rust, similar to a TypeScript/JavaScript "service class" you might use in React/Node.
// It uses async code and provides typed wrappers for API endpoints. If you're familiar with axios or fetch in JS, reqwest is the Rust equivalent.
//
// Throughout, I'll add comments explaining Rust syntax and concepts in comparison to JS/TS.

use anyhow::{Context, Result}; // 'anyhow' is a Rust crate providing error handling; 'Result' is Rust's equivalent to Promise<Result<T, E>>
use reqwest::Client; // Reqwest is like 'fetch' or 'axios' in JS/TS for HTTP requests
use serde::Deserialize; // Serde handles mapping (deserialization) of JSON responses to Rust structs

use crate::models::{ // This brings in some types for request/response payloads that were defined elsewhere
    ApiGame, CreatePvpRequest, CreateSoloRequest, JoinPvpRequest, PlayMoveRequest,
};

// ==============================
// API Client Struct Declaration
// ==============================
// In Rust, structs are like classes but only contain data. Methods are added in an 'impl' (implementation) block.
// Here, we're defining a struct that wraps an HTTP client and a base URL.
// In TS: interface ApiClient { client: AxiosInstance; baseUrl: string }
pub struct ApiClient {
    client: Client,
    base_url: String,
}

// ====================================
// Implementation of ApiClient Methods
// ====================================
// In Rust, 'impl' blocks are used to add methods to structs. Methods can take &self (reference to the struct instance, sort of like 'this' in JS).
impl ApiClient {
    // Constructor: like 'new ApiClient(baseUrl)' in JS/TS
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(), // creates a new HTTP client
            base_url: base_url.to_string(), // converts &str (string slice) to String
        }
    }

    // ===============================
    // Endpoint: Create Solo Game
    // ===============================
    // Async function (like async in JS/TS), returns Result<ApiGame, Error>
    pub async fn create_solo_game(&self, player_id: &str) -> Result<ApiGame> {
        let url = format!("{}/games/solo", self.base_url); // build the endpoint URL
        let payload = CreateSoloRequest {
            player_id: player_id.to_string(), // convert to String
            client_name: "rust-tui-client".to_string(), // hardcoded name for client
        };

        // Make a POST request, serialize payload to JSON, wait for response
        let response = self.client.post(url).json(&payload).send().await?;
        // Custom function to parse response as JSON and handle errors
        parse_json_response(response).await
    }

    // ===============================
    // Endpoint: Create PvP Game
    // ===============================
    pub async fn create_pvp_game(
        &self,
        player_id: &str,
        name: &str,
        password: Option<String>, // Option<T> is like T | undefined/null in TS
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

    // ===============================
    // Endpoint: List Open PvP Games
    // ===============================
    pub async fn list_open_pvp_games(&self) -> Result<Vec<ApiGame>> {
        let url = format!("{}/games/pvp/open", self.base_url);
        let response = self.client.get(url).send().await?;
        parse_json_response(response).await
    }

    // ===============================
    // Endpoint: Join PvP Game
    // ===============================
    pub async fn join_pvp_game(
        &self,
        player_id: &str,
        game_id: &str,
        password: Option<String>,
    ) -> Result<ApiGame> {
        let url = format!("{}/games/pvp/{game_id}/join", self.base_url); // Format strings in Rust use curly braces, like template literals
        let payload = JoinPvpRequest {
            player_id: player_id.to_string(),
            password,
        };

        let response = self.client.post(url).json(&payload).send().await?;
        parse_json_response(response).await
    }

    // ===============================
    // Endpoint: Get Single Game
    // ===============================
    pub async fn get_game(&self, game_id: &str) -> Result<ApiGame> {
        let url = format!("{}/games/{game_id}", self.base_url);
        let response = self.client.get(url).send().await?;
        parse_json_response(response).await
    }

    // ===============================
    // Endpoint: Play Move
    // ===============================
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

// ===============================
// Helper Function: Parse Response
// ===============================
// This takes the HTTP response, checks if the status is success, and parses JSON to the expected type.
// In TS, you'd do: if (!response.ok) throw Error()
// 'anyhow' is used for error reporting; .context() annotates errors for easier debugging
async fn parse_json_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T> {
    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());
        anyhow::bail!("request failed with {status}: {body}"); // bail = throw error
    }

    response
        .json::<T>()
        .await
        .context("invalid JSON response shape") // attaches error context/history
}

// ===============================
// Summary
// ===============================
// This Rust module is a direct analog to a TS service file using axios/fetch.
// - Struct = Typed object/class
// - impl = implementation of methods, added to struct
// - Result<T> = Promise<Result<T, E>>
// - async/await works as expected, but with Rust's error handling
// - Option<T> = T | undefined/null
// - .to_string() = String(obj)
// - Format macros and string interpolation = TS template literals
// - Custom error reporting for debugging
//
// If you want to understand Rust, map things to TS as above. You can call these methods from elsewhere in the app, just like you'd call service functions in React/Node.
