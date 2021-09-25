use tracing::{debug, info};
use serde::{Deserialize, Serialize};
use axum::extract::Json;
use axum::response::IntoResponse;
use axum::http::{Request, header::HeaderMap, StatusCode};
use axum::extract::ConnectInfo;
use std::net::SocketAddr;
use rand::Rng;
use serde_json::json;
use std::str::from_utf8;
use std::io::{BufReader, BufRead};
use std::collections::HashMap;
use reqwest::header::CONTENT_TYPE;

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
        } else {
            self.board[i][j] = self.turn.clone();
        }
        self.next_turn();
    }
}

#[tracing::instrument]
pub(crate) async fn arena_handle(payload: String) -> Result<(), StatusCode> {
    info!("called");
    let payload: TicTacToePostRequest = serde_json::from_str(&payload).map_err(|_e| StatusCode::BAD_REQUEST)?;
    let battle_id = payload.battle_id.clone();

    std::thread::spawn(move || -> Result<(), StatusCode> {
        let end_point = format!("https://cis2021-arena.herokuapp.com/tic-tac-toe/start/{}", battle_id);
        debug!("{}", end_point);

        let mut game = None;
        let event_source = reqwest::blocking::Client::new();
        let mut res = event_source.get(&end_point).send().unwrap();
        let mut reader = BufReader::new(&mut res);
        loop {
            let mut buf= String::new();
            match reader.read_line((&mut buf)) {
                Ok(x) if x <=5 => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Ok(_) => {
                    if let Ok(initial_event) = serde_json::from_str::<InitialEvent>(&buf[5..]) {
                        debug!("buf={}, event={:?}", buf, initial_event);
                        game = Some(TicTacToe::new(&initial_event.you_are));

                        if game.as_ref().unwrap().is_my_turn() {
                            let new_pos = game.as_mut().unwrap().random_move();

                            let response = serde_json::to_string(&json!({
                                "action": "putSymbol",
                                "position": new_pos
                            })).unwrap();
                            debug!("{}", response);

                            // send response
                            let mut response = HashMap::new();
                            response.insert("action", "putSymbol");
                            response.insert("position", &new_pos);
                            let client = reqwest::blocking::Client::new();
                            let res = client
                                .post(format!("https://cis2021-arena.herokuapp.com/tic-tac-toe/play/{}", battle_id))
                                .header(CONTENT_TYPE, "application/json")
                                .json(&response)
                                .send();
                            // debug!("{:?}", &res);
                        }
                    } else if let Ok(move_event) = serde_json::from_str::<MoveEvent>(&buf[5..]) {
                        debug!("buf={}, event={:?}", buf, move_event);

                        if move_event.player == game.as_ref().unwrap().player {
                            continue
                        }

                        let pos = move_event.position.clone();
                        game.as_mut().unwrap().play_symbol(pos);
                        debug!("{:?}", game);

                        let new_pos = game.as_mut().unwrap().random_move();
                        debug!("{:?}", &game);
                        let mut response = HashMap::new();
                        response.insert("action", "putSymbol");
                        response.insert("position", &new_pos);
                        let client = reqwest::blocking::Client::new();
                        let res = client
                            .post(format!("https://cis2021-arena.herokuapp.com/tic-tac-toe/play/{}", battle_id))
                            .header(CONTENT_TYPE, "application/json")
                            .json(&response)
                            .send();
                        // debug!("{:?}", &res);
                    } else if let Ok(game_end_event) = serde_json::from_str::<GameEndEvent>(&buf[5..]) {
                        debug!("buf={}, event={:?}", buf, game_end_event);
                        break;
                    } else if let Ok(flip_table_event) = serde_json::from_str::<FlipTableEvent>(&buf[5..]) {
                        debug!("buf={}, event={:?}", buf, flip_table_event);
                        break;
                    } else {
                        debug!("known buf={}", buf);
                    }
                }
                Err(_) => { break; }
            }
        }

        Ok(())
    }).join();
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