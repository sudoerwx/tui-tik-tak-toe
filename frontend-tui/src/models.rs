use serde::{Deserialize, Serialize};

// Mirrors backend game JSON shape.
// Think of this like a TypeScript interface used in API responses.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiGame {
    pub id: String,
    pub mode: String,
    pub name: Option<String>,
    #[serde(rename = "hostPlayerId")]
    pub host_player_id: String,
    #[serde(rename = "guestPlayerId")]
    pub guest_player_id: Option<String>,
    pub board: Vec<Option<String>>,
    #[serde(rename = "currentTurn")]
    pub current_turn: String,
    pub status: String,
    pub winner: Option<String>,
    #[serde(rename = "hasPassword")]
    pub has_password: bool,
}

#[derive(Debug, Serialize)]
pub struct CreateSoloRequest {
    #[serde(rename = "playerId")]
    pub player_id: String,
    #[serde(rename = "clientName")]
    pub client_name: String,
}

#[derive(Debug, Serialize)]
pub struct CreatePvpRequest {
    #[serde(rename = "playerId")]
    pub player_id: String,
    pub name: String,
    pub password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct JoinPvpRequest {
    #[serde(rename = "playerId")]
    pub player_id: String,
    pub password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlayMoveRequest {
    #[serde(rename = "playerId")]
    pub player_id: String,
    pub index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    SoloGame,
    PvpLobby,
    PvpCreate,
    PvpGame,
    GameOver,
    Info,
}
