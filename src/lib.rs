use ciphercore_base::data_values::Value;
use ciphercore_base::errors::Result;
use ciphercore_base::evaluators::simple_evaluator::SimpleEvaluator;
use ciphercore_base::graphs::{Node, NodeAnnotation, Operation};
use ciphercore_base::random::SEED_SIZE;
use std::sync::mpsc::{Receiver, Sender};

pub struct Channels {
    tx_next: Sender<String>,
    rx_next: Receiver<String>,
    tx_prev: Sender<String>,
    rx_prev: Receiver<String>,
}

impl Channels {
    pub fn new(
        tx_next: Sender<String>,
        rx_next: Receiver<String>,
        tx_prev: Sender<String>,
        rx_prev: Receiver<String>,
    ) -> Self {
        Channels {
            tx_next,
            rx_next,
            tx_prev,
            rx_prev,
        }
    }
}

pub struct Evaluator {
    party: u64,
    channels: Channels,
    simple: SimpleEvaluator,
}

impl Evaluator {
    pub fn new(party: u64, channels: Channels, prng_seed: Option<[u8; SEED_SIZE]>) -> Result<Self> {
        Ok(Evaluator {
            party,
            channels,
            simple: SimpleEvaluator::new(prng_seed).unwrap(),
        })
    }
}

impl ciphercore_base::evaluators::Evaluator for Evaluator {
    fn evaluate_node(&mut self, node: Node, dependencies_values: Vec<Value>) -> Result<Value> {
        let mut val = self
            .simple
            .evaluate_node(node.clone(), dependencies_values)
            .unwrap();
        if let Operation::NOP = node.get_operation() {
            for annotation in node.get_annotations().unwrap() {
                if let NodeAnnotation::Send(sender_id, receiver_id) = annotation {
                    if sender_id == self.party {
                        let tx = if receiver_id == (self.party + 1) % 3 {
                            &self.channels.tx_next
                        } else {
                            &self.channels.tx_prev
                        };
                        tx.send(serde_json::to_string(&val).unwrap()).unwrap();
                    }
                    if receiver_id == self.party {
                        let rx = if sender_id == (self.party + 1) % 3 {
                            &self.channels.rx_next
                        } else {
                            &self.channels.rx_prev
                        };
                        val = serde_json::from_str::<Value>(rx.recv().unwrap().as_str()).unwrap();
                    }
                }
            }
        }
        Ok(val)
    }
}
