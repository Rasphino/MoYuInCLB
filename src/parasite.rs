use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};
use axum::Json;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug)]
pub struct ParasitePostRequest {
    room: usize,
    grid: Vec<Vec<u32>>,
    #[serde(rename = "interestedIndividuals")]
    indi: Vec<String>
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ParasitePostResponse {
    room: usize,
    p1: Value,
    p2: i32,
    p3: i32,
    p4: i32
}


pub(crate) async fn parasite_handle(payload: String) -> Result<Json<Vec<ParasitePostResponse>>, StatusCode>  {
    debug!("called");
    let payload: Vec<ParasitePostRequest> = serde_json::from_str(&payload).map_err(|_e| StatusCode::BAD_REQUEST)?;
    debug!("payload={:?}", payload);

    let resps = payload.into_iter()
        .map(|mut req| {
            let ticks_map = Parasite::bfs4(&mut req.grid);
            let mut p1 = serde_json::Map::new();
            let indi = req.indi.clone();
            for ind in indi {
                let x = ind.split(',').map(|s| s.parse::<usize>().unwrap()).collect::<Vec<_>>();
                let i = x[0];
                let j = x[1];
                p1.insert(ind, Value::from(ticks_map[i][j]));
            }
            let p2 = {
                let mut max = -1;
                for i in 0..ticks_map.len() {
                    for j in 0..ticks_map[0].len() {
                        if ticks_map[i][j] == -1 && req.grid[i][j] == 1 {
                            max = -1;
                            break;
                        } else {
                            max = std::cmp::max(max, ticks_map[i][j]);
                        }
                    }
                }
                max
            };
            let p3 = 1;
            let p4 = 1;
            ParasitePostResponse {
                room: req.room,
                p1: Value::Object(p1),
                p2, p3, p4
            }
        })
        .collect::<>();

    Ok(Json(resps))
}

struct Parasite {

}

impl Parasite {
    fn bfs4(map: &mut Vec<Vec<u32>>) -> Vec<Vec<i32>> {
        let mut ticks = 0;
        let m = map.len();
        let n = map[0].len();
        let mut tick_map = vec![vec![-1; n]; m];

        let mut list = Vec::new();
        for i in 0..m {
            for j in 0..n {
                if map[i][j] == 3 {
                    list.push((i, j));
                    tick_map[i][j] = 0;
                }
            }
        }

        while !list.is_empty() {
            ticks += 1;
            let (i, j) = list.pop().unwrap();
            let directions = [(-1, 0), (1, 0), (0, -1), (0, 1)];
            for (di, dj) in directions {
                let (mut i, mut j) = (i as i32, j as i32);
                i += di;
                j += dj;
                if i < 0 || i >= m as i32 || j < 0 || j >= n as i32 { continue; }
                let (i, j) = (i as usize, j as usize);
                match map[i][j] {
                    1 => {
                        list.push((i, j));
                        map[i][j] = 3;
                        tick_map[i][j] = ticks;
                    },
                    2 => {
                        list.push((i, j));
                    }
                    _ => {}
                }
            }
        }

        tick_map
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dfs4() {
        let mut map = vec![
            vec![0, 3, 2],
            vec![0, 1, 1],
            vec![1, 0, 0]
        ];
        let ticks_map = Parasite::bfs4(&mut map);

        assert_eq!(map, vec![
            vec![0, 3, 2],
            vec![0, 3, 3],
            vec![1, 0, 0]
        ]);
        assert_eq!(ticks_map, vec![
            vec![-1, 0, -1],
            vec![-1, 1, 2],
            vec![-1, -1, -1]
        ])
    }
}