use tracing::{debug, info};
use serde::{Deserialize, Serialize};
use axum::extract::Json;
use axum::response::IntoResponse;
use axum::http::{Request, header::HeaderMap, StatusCode};
use axum::extract::ConnectInfo;
use std::net::SocketAddr;
use sse_client::EventSource;
use rand::Rng;
use serde_json::json;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct TicTacToePostRequest {
    #[serde(rename = "battleId")]
    battle_id: String
}

#[derive(Deserialize, Serialize, Debug)]
struct InitialEvent {
    #[serde(rename = "youAre")]
    you_are: String,
    id: String
}

#[derive(Deserialize, Serialize, Debug)]
struct MoveEvent {
    player: String,
    action: String,
    position: String
}

#[derive(Deserialize, Serialize, Debug)]
struct GameEndEvent {
    winner: String
}

#[derive(Deserialize, Serialize, Debug)]
struct FlipTableEvent {
    player: String,
    action: String
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TicTacToe {
    player: String,
    turn: String,
    board: Vec<Vec<String>>
}

impl TicTacToe {
    pub fn new(player: &str) -> Self {
        TicTacToe {
            player: player.to_owned(),
            turn: "O".to_string(),
            board: vec![vec![" ".to_string(); 3]; 3]
        }
    }

    fn is_my_turn(&self) -> bool {
        self.turn == self.player
    }

    fn index_to_position(i: usize, j: usize) -> String {
        match (i, j) {
            (0, 0) => "NW".to_string(),
            (0, 1) => "N".to_string(),
            (0, 2) => "NE".to_string(),
            (1, 0) => "W".to_string(),
            (1, 1) => "C".to_string(),
            (1, 2) => "E".to_string(),
            (2, 0) => "SW".to_string(),
            (2, 1) => "S".to_string(),
            (2, 2) => "SE".to_string(),
            _ => unreachable!()
        }
    }

    fn position_to_index(pos: &str) -> (usize, usize) {
        match pos {
            "NW" => (0, 0),
            "N" => (0, 1),
            "NE" => (0, 2),
            "W" => (1, 0),
            "C" => (1, 1),
            "E" => (1, 2),
            "SW" => (2, 0),
            "S" => (2, 1),
            "SE" => (2, 2),
            _ => unreachable!()
        }
    }

    fn random_move(&mut self) -> String {
        use rand::prelude::*;

        let mut rng = thread_rng();
        let mut i = rng.gen_range(0..3);
        let mut j = rng.gen_range(0..3);
        while self.board[i][j] != " " {
            i = rng.gen_range(0..3);
            j = rng.gen_range(0..3);
        }
        self.board[i][j] = self.turn.clone();
        self.next_turn();
        Self::index_to_position(i, j)
    }

    fn next_turn(&mut self) {
        if self.turn == "O" {
            self.turn = "X".to_string();
        } else {
            self.turn = "O".to_string();
        }
    }

    fn play_symbol(&mut self, pos: String) {
        let (i, j) = Self::position_to_index(&pos);
        if self.board[i][j] != " " {
            // todo: flip the table
        }
        self.board[i][j] = self.turn.clone();
        self.next_turn();
    }
}

#[tracing::instrument]
pub(crate) async fn arena_handle(ConnectInfo(addr): ConnectInfo<SocketAddr>, payload: String) -> Result<(), StatusCode> {
    info!("called");
    let payload: TicTacToePostRequest = serde_json::from_str(&payload).map_err(|_e| StatusCode::BAD_REQUEST)?;
    let battle_id = payload.battle_id.clone();
    debug!("{}", &battle_id);

    tokio::task::spawn_blocking(move || -> Result<(), StatusCode> {
        let event_source = EventSource::new(&format!("http://{}/tic-tac-toe/start/{}", addr, battle_id)).unwrap();
        let rx = event_source.receiver();
        let initial_event = rx.recv().map_err(|_e| StatusCode::BAD_REQUEST)?;
        let initial_event: InitialEvent = serde_json::from_str(&initial_event.data).map_err(|_e| StatusCode::BAD_REQUEST)?;
        let mut game = TicTacToe::new(&initial_event.you_are);
        debug!("{:?}", &game);
        if game.is_my_turn() {
            let new_pos = game.random_move();
            debug!("{:?}", &game);
            let response = json!({
                "action": "putSymbol",
                "position": new_pos
            }).to_string();
            // send response
            let client = reqwest::blocking::Client::new();
            let res = client
                .post(format!("http://{}/tic-tac-toe/play/{}", addr, battle_id))
                .body(response)
                .send();
            debug!("{:?}", &res);
        }

        for event in rx.iter() {
            if let Ok(move_event) = serde_json::from_str::<MoveEvent>(&event.data) {
                debug!("{:?}", &move_event);
                let pos = move_event.position.clone();
                game.play_symbol(pos);
                debug!("{:?}", &game);

                let new_pos = game.random_move();
                debug!("{:?}", &game);
                let response = json!({
                "action": "putSymbol",
                "position": new_pos
            }).to_string();

                let client = reqwest::blocking::Client::new();
                let res = client
                    .post(format!("http://{}/tic-tac-toe/play/{}", addr, battle_id))
                    .body(response)
                    .send();
                debug!("{:?}", &res);
            }
            if let Ok(game_end_event) = serde_json::from_str::<GameEndEvent>(&event.data) {
                debug!("{:?}", &game_end_event);
                break;
            }
            if let Ok(flip_table_event) = serde_json::from_str::<FlipTableEvent>(&event.data) {
                debug!("{:?}", &flip_table_event);
                break;
            }
        }
        Ok(())
    }).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_request() {
        let req = r#"
{
  "battleId": "21083c13-f0c2-4b54-8cb1-090129ffaa93"
}
        "#;
        assert_eq!(TicTacToePostRequest { battle_id: "21083c13-f0c2-4b54-8cb1-090129ffaa93".to_string() }, serde_json::from_str::<TicTacToePostRequest>(req).unwrap());
    }
}