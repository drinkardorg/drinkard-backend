use crate::{authentication::AuthenticationData, db::Db};
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt::Display, sync::Arc, time::Instant};
use tokio::sync::{
    mpsc::{self, UnboundedSender},
    Mutex,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::{filters::ws::Message, reply::Reply};

#[derive(Default, Debug)]
pub struct State {
    pub games: Vec<GameState>,
    pub pending_match: Option<(String, UnboundedSender<Message>)>,
}

impl State {
    pub fn new() -> Self {
        Self {
            games: Vec::new(),
            pending_match: None,
        }
    }

    pub fn get_game(&mut self, id: &str) -> &mut GameState {
        self.games
            .iter_mut()
            .find(|x| x.p1.0 == id || x.p2.0 == id)
            .unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Symbol {
    #[default]
    Clubs,
    Spades,
    Diamonds,
    Hearts,
}

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Symbol::Clubs => f.write_str("clubs"),
            Symbol::Spades => f.write_str("spades"),
            Symbol::Diamonds => f.write_str("diamonds"),
            Symbol::Hearts => f.write_str("hearts"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Card {
    pub value: u32,
    pub symbol: Symbol,
}

impl Card {
    pub fn normalize(&mut self) {
        // ace
        if self.value == 1 {
            self.value = 14;
        }
    }
}

#[derive(Debug)]
pub struct GameState {
    pub p1: (String, UnboundedSender<Message>),
    pub p2: (String, UnboundedSender<Message>),
    pub turn: String,
    pub timer: Instant,
    pub p1_cards: Vec<Card>,
    pub p2_cards: Vec<Card>,
    pub turned_card: Option<Card>,
}

#[derive(Serialize, Deserialize)]
pub struct GameStartNotification {
    id: String,
    cards: Vec<Card>,
    opponent_elo: i32,
    opponent_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct GameTurnNotification {
    id: String,
    turn: String,
}

#[derive(Serialize, Deserialize)]
pub struct TurnedCardNotif {
    id: String,
    value: i32,
    symbol: String,
}

#[derive(Serialize, Deserialize)]
pub struct GameEndNotification {
    id: String,
    winner: String,
}

pub async fn handle(db: Arc<Db>, state: Arc<Mutex<State>>, ws: warp::ws::WebSocket) {
    let (tx, rx) = mpsc::unbounded_channel();
    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    let authentication = match user_ws_rx.next().await {
        Some(Ok(authentication)) => authentication,
        _ => {
            log::error!("Invalid authentication");
            return;
        }
    };
    let authentication =
        match serde_json::from_slice::<AuthenticationData>(authentication.as_bytes()) {
            Ok(authentication) => authentication,
            Err(_) => {
                log::error!("Invalid authentication json");
                return;
            }
        };

    let me = match db
        .get_user_by_name_password(&authentication.username, &authentication.password)
        .await
    {
        Ok(user) => user,
        Err(error) => {
            eprintln!("Invalid authentication for /game: {}", error);
            return;
        }
    };

    let mut rx = UnboundedReceiverStream::new(rx);
    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            user_ws_tx
                .send(message)
                .unwrap_or_else(|e| {
                    eprintln!("websocket send error: {}", e);
                })
                .await;
        }
    });

    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error (username: {}): {}", me.username, e);
                continue;
            }
        };

        if !msg.is_text() {
            continue;
        }

        let msg = match serde_json::from_slice::<Value>(msg.as_bytes()) {
            Ok(msg) => msg,
            Err(error) => {
                eprintln!(
                    "websocket parse error (username: {}): {}",
                    me.username, error
                );
                continue;
            }
        };

        let id = msg["id"].as_str().unwrap();
        match id {
            "findmatch" => {
                let mut state = state.lock().await;
                if let Some(pending_match) = state.pending_match.take() {
                    let (p1_cards, p2_cards, turn) = {
                        let mut rand = rand::thread_rng();
                        let mut cards = Vec::with_capacity(52);

                        for symbol in [
                            Symbol::Clubs,
                            Symbol::Diamonds,
                            Symbol::Hearts,
                            Symbol::Spades,
                        ]
                        .into_iter()
                        {
                            for i in 1..=13 {
                                cards.push(Card { symbol, value: i });
                            }
                        }

                        cards.shuffle(&mut rand);

                        let mut p1_cards = Vec::with_capacity(26);
                        let mut p2_cards = Vec::with_capacity(26);

                        for (i, card) in cards.into_iter().enumerate() {
                            if i % 2 == 0 {
                                p1_cards.push(card);
                            } else {
                                p2_cards.push(card);
                            }
                        }

                        (p1_cards, p2_cards, rand.gen_bool(0.5))
                    };

                    let opponent = db.get_user_by_name(&pending_match.0).await.unwrap();
                    let notif1 = GameStartNotification {
                        id: String::from("gamestart"),
                        cards: p2_cards.clone(),
                        opponent_elo: opponent.elo_points,
                        opponent_name: opponent.username,
                    };

                    let notif2 = GameStartNotification {
                        id: String::from("gamestart"),
                        cards: p1_cards.clone(),
                        opponent_elo: me.elo_points,
                        opponent_name: me.username.clone(),
                    };

                    tx.send(Message::text(serde_json::to_string(&notif1).unwrap()))
                        .unwrap();

                    pending_match
                        .1
                        .send(Message::text(serde_json::to_string(&notif2).unwrap()))
                        .unwrap();

                    let turn_notif = if turn {
                        GameTurnNotification {
                            id: String::from("turnnotif"),
                            turn: pending_match.0.clone(),
                        }
                    } else {
                        GameTurnNotification {
                            id: String::from("turnnotif"),
                            turn: me.username.clone(),
                        }
                    };

                    let turn_notif = Message::text(serde_json::to_string(&turn_notif).unwrap());

                    tx.send(turn_notif.clone()).unwrap();
                    pending_match.1.send(turn_notif).unwrap();

                    let game = GameState {
                        turn: if turn {
                            pending_match.0.clone()
                        } else {
                            me.username.clone()
                        },
                        p1: pending_match,
                        p2: (me.username.clone(), tx.clone()),
                        timer: Instant::now(),
                        p1_cards,
                        p2_cards,
                        turned_card: None,
                    };

                    state.games.push(game);
                } else {
                    state.pending_match = Some((me.username.clone(), tx.clone()));
                }
            }

            "turncard" => {
                let mut state = state.lock().await;
                let game = state.get_game(&me.username);
                if game.turn != me.username {
                    continue;
                }

                let symbol = msg["cardsymbol"].as_str().unwrap();
                let value = msg["cardvalue"].as_i64().unwrap() as u32;
                let symbol = match symbol {
                    "spades" => Symbol::Spades,
                    "clubs" => Symbol::Clubs,
                    "diamonds" => Symbol::Diamonds,
                    _ => Symbol::Hearts,
                };

                let is_p1 = me.username == game.p1.0;
                let cards = if is_p1 {
                    &mut game.p1_cards
                } else {
                    &mut game.p2_cards
                };

                let index = cards
                    .iter()
                    .position(|x| x.symbol == symbol && x.value == value)
                    .unwrap();

                let mut turned_card = cards.remove(index);

                if let Some(mut prev_turned_card) = game.turned_card.take() {
                    turned_card.normalize();
                    prev_turned_card.normalize();

                    if turned_card.value < prev_turned_card.value {
                        if is_p1 {
                            game.turn = game.p2.0.clone();
                        } else {
                            game.turn = game.p1.0.clone();
                        }
                    }
                } else {
                    if is_p1 {
                        if game.p2_cards.iter().all(|x| x.symbol != turned_card.symbol) {
                            game.turn = game.p1.0.clone();
                        } else {
                            game.turn = game.p2.0.clone();
                            game.turned_card = Some(turned_card);
                        }
                    } else {
                        if game.p1_cards.iter().all(|x| x.symbol != turned_card.symbol) {
                            game.turn = game.p2.0.clone();
                        } else {
                            game.turn = game.p1.0.clone();
                            game.turned_card = Some(turned_card);
                        }
                    }
                }

                let turned_card_notif = if game.turned_card.is_some() {
                    TurnedCardNotif {
                        id: String::from("turnedcardnotif"),
                        value: turned_card.value as i32,
                        symbol: turned_card.symbol.to_string(),
                    }
                } else {
                    TurnedCardNotif {
                        id: String::from("turnedcardnotif"),
                        value: 0,
                        symbol: String::new(),
                    }
                };

                if is_p1 {
                    game.p2
                        .1
                        .send(Message::text(
                            serde_json::to_string(&turned_card_notif).unwrap(),
                        ))
                        .unwrap();
                } else {
                    game.p1
                        .1
                        .send(Message::text(
                            serde_json::to_string(&turned_card_notif).unwrap(),
                        ))
                        .unwrap();
                }

                if game.p1_cards.is_empty() || game.p1_cards.is_empty() {
                    let end_notif = GameEndNotification {
                        id: String::from("gameend"),
                        winner: game.turn.clone(),
                    };

                    let end_notif = Message::text(serde_json::to_string(&end_notif).unwrap());

                    db.add_elo(&me.username, 15).await;

                    if is_p1 {
                        db.remove_elo(&game.p2.0, 15).await;
                    } else {
                        db.remove_elo(&game.p1.0, 15).await;
                    }

                    game.p1.1.send(end_notif.clone()).unwrap();
                    game.p2.1.send(end_notif).unwrap();

                    let index = state
                        .games
                        .iter()
                        .position(|x| x.p1.0 == me.username || x.p2.0 == me.username)
                        .unwrap();
                    state.games.remove(index);
                } else {
                    let turn_notif = GameTurnNotification {
                        id: String::from("turnnotif"),
                        turn: game.turn.clone(),
                    };

                    let turn_notif = Message::text(serde_json::to_string(&turn_notif).unwrap());
                    game.p1.1.send(turn_notif.clone()).unwrap();
                    game.p2.1.send(turn_notif).unwrap();
                }
            }

            _ => {}
        }
    }
}

pub fn game(db: Arc<Db>, state: Arc<Mutex<State>>, ws: warp::ws::Ws) -> impl Reply {
    ws.on_upgrade(|websocket| handle(db, state, websocket))
}
