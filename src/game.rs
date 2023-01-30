use std::borrow::{Borrow, BorrowMut};
use std::thread::current;
use crate::card::{Card, Deck};
use crate::player::Player;

pub struct GameState<'a> {
    deck: Deck,
    discard: Vec<Card>,
    players: Vec<(&'a mut dyn Player, Vec<Card>)>,
    current_player: usize,
    direction: Direction,
    to_draw: u8,
}

pub struct Turn<'a> {
    pub to_draw: u8,
    pub hand: &'a mut Vec<Card>,
    draw_pile: &'a mut Deck,
    discard_pile: &'a mut Vec<Card>,
}

pub enum TurnResult {
    Played(Card),
    Drew,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Direction {
    Clockwise,
    CounterClockwise,
}


impl<'a> GameState<'a> {
    pub fn new(players: Vec<&'a mut dyn Player>) -> GameState<'a> {
        GameState {
            deck: Deck::generate(),
            discard: vec![],
            players: players.into_iter().map(|p| (p, vec![])).collect(),
            current_player: 0,
            direction: Direction::Clockwise,
            to_draw: 0,
        }
    }

    pub fn start(&mut self) -> ! {
        self.deck.shuffle();

        for (_, hand) in self.players.iter_mut() {

            let insert = self.deck.draw_multiple(7);
            hand.extend(insert);
        }

        loop {
            let top_card = self.deck.draw().unwrap();

            match top_card {
                Card::Wild { color: _ } => {
                    self.deck.reinsert_random(top_card);
                }
                Card::DrawFour { color: _ } => {
                    self.deck.reinsert_random(top_card);
                }
                _ => {
                    self.discard.push(top_card);
                    println!("The top card is: {}", self.discard.last().unwrap());
                    break;
                }
            }
        }

        loop {
            std::thread::sleep(std::time::Duration::from_millis(800));

            Self::ensure_drawable_deck(&mut self.deck, &mut self.discard, self.to_draw);

            self.current_player = self.next_player();

            // Play for the current player
            let (current_player, player_hand) = self.players.get_mut(self.current_player).unwrap();

            let playable_player_hand = &mut Self::get_playable_hand(player_hand, self.discard.last().unwrap(), self.to_draw);

            if self.to_draw > 0 && !Self::contains_special_card(playable_player_hand, self.discard.last().unwrap()) {
                let draw = &self.deck.draw_multiple(self.to_draw);

                player_hand.extend(draw);
                current_player.observe_turn_skip(Some(draw.iter().collect()));

                println!("{} drew {} cards ({} cards in deck, {} in discard)", current_player.name(), self.to_draw, self.deck.cards.len(), self.discard.len());

                self.to_draw = 0;
                continue;
            }

            let turn = Turn {
                hand: playable_player_hand,
                draw_pile: &mut self.deck,
                discard_pile: &mut self.discard,
                to_draw: self.to_draw,
            };

            match current_player.execute_turn(&turn) {
                TurnResult::Played(card) => {
                    player_hand.remove(player_hand.iter().position(|c| *c == card).unwrap());
                    self.discard.push(card);

                    println!("{} played {}", current_player.name(), card);

                    match card {
                        Card::Skip { .. } => {

                            self.current_player = self.next_player();
                            let next_player = &self.players.get_mut(self.current_player).unwrap().0;

                            next_player.observe_turn_skip(None);

                            println!("{}'s turn was skipped", next_player.name());
                            continue;
                        }
                        Card::Reverse { .. } => {
                            self.direction = match self.direction {
                                Direction::Clockwise => Direction::CounterClockwise,
                                Direction::CounterClockwise => Direction::Clockwise,
                            };
                        }
                        Card::DrawTwo { .. } => {
                            self.to_draw += 2;
                        }
                        Card::DrawFour { .. } => {
                            self.to_draw += 4;
                        }
                        _ => {}
                    }
                }
                TurnResult::Drew => {
                    if self.to_draw == 0 {
                        self.to_draw += 1;
                    }

                    let cards = &self.deck.draw_multiple(self.to_draw);
                    player_hand.extend(cards);

                    current_player.observe_turn_skip(Some(cards.iter().collect()));

                    println!("{} drew {} card(s)", current_player.name(), cards.len());

                    self.to_draw = 0;
                }
            };

            if player_hand.is_empty() {
                println!("{} won!", current_player.name());
                std::thread::sleep(std::time::Duration::from_millis(4500));
                std::process::exit(0);
            }
        }
    }

    fn get_playable_hand(hand: &[Card], card: &Card, to_draw: u8) -> Vec<Card> {

        if to_draw > 0 && matches!(card, Card::DrawTwo { .. } | Card::DrawFour { .. }) {
            return hand.iter().filter(|c| **c == *card).copied().collect::<Vec<Card>>();
        }

        hand.iter()
            .filter(|c| c.can_play_on(card))
            .copied()
            .collect::<Vec<Card>>()
    }

    fn contains_special_card(hand: &[Card], card: &Card) -> bool {
        hand.iter().any(|c| *c == *card)
    }

    fn ensure_drawable_deck(deck: &mut Deck, discard: &mut Vec<Card>, to_draw: u8) {
        if (deck.cards.len() as u8) >= to_draw {
            return;
        }

        if (discard.len() as u8) >= to_draw {
            let from_discard = discard.drain(..discard.len());
            deck.cards.extend(from_discard);
            deck.shuffle();

        } else { // Should this be a panic case?
            discard.drain(..discard.len()); // Keep the last card

            // push a supplementary deck
            let mut new_deck = Deck::generate();
            deck.cards.extend(new_deck.cards);

            deck.shuffle();
        }
    }

    fn next_player(&self) -> usize{
        let mut index = self.current_player;
        let direction = self.direction;

        match direction {
            Direction::Clockwise => {
                index = (index + 1) % self.players.len()
            },
            Direction::CounterClockwise => {

                if index == 0 {
                    index = self.players.len() - 1;
                } else {
                    index -= 1;
                }
            }
        };

        index
    }
}