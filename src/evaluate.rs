//! $ cargo run --bin evaluate -- data/sum.json data/sum_input.json 0,1,2  2>/tmp/err
//! expected output:
//!   Party 0: got output {"kind":"scalar","type":"i32","value":60}
//!   Party 1: got output {"kind":"scalar","type":"i32","value":60}
//!   Party 2: got output {"kind":"scalar","type":"i32","value":60}
//!
//! $ cargo run --bin evaluate -- data/matmul.json data/matmul_input.json 0,1  2>/tmp/err
//! expected output:
//!   Party 0: got output <random_garbage>
//!   Party 1: got output <random_garbage>
//!   Party 2: got output {"kind":"array","type":"i64","value":[[515,530,545,560,575],[1290,1330,1370,1410,1450],[2065,2130,2195,2260,2325],[2840,2930,3020,3110,3200],[3615,3730,3845,3960,4075]]}

use ciphercore_base::data_values::Value;
use ciphercore_base::errors::Result;
use ciphercore_base::evaluators::get_result_util::get_evaluator_result;
use ciphercore_base::graphs::Context;
use ciphercore_base::mpc::mpc_compiler::IOStatus;
use ciphercore_base::runtime_error;
use ciphercore_base::typed_value::TypedValue;
use ciphercore_utils::execute_main::execute_main;
use multi_parti_evaluator::Channels;
use multi_parti_evaluator::Evaluator;
use std::fs;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about=None)]
struct Args {
    #[clap(value_parser)]
    /// Path to a file containing serialized context.
    context_path: String,
    #[clap(value_parser)]
    /// Path to a file containing serialized inputs.
    inputs_path: String,
    /// String comprising comma separated list of input parties' IDs, valid ID values include `0`, `1`, `2` OR `public` OR `secret-shared`.
    input_parties: String,
}

fn get_tokens(s: String) -> Result<Vec<IOStatus>> {
    let tokens: Vec<String> = s.split(',').map(|x| x.to_owned()).collect();
    if tokens.is_empty() {
        return Err(runtime_error!("Empty tokens"));
    }
    let mut result = Vec::new();
    for token in tokens {
        match token.as_str() {
            "0" => result.push(IOStatus::Party(0)),
            "1" => result.push(IOStatus::Party(1)),
            "2" => result.push(IOStatus::Party(2)),
            "public" => result.push(IOStatus::Public),
            "secret-shared" => result.push(IOStatus::Shared),
            _ => return Err(runtime_error!("Invalid token: {}", token)),
        }
    }
    Ok(result)
}

fn parse_input_parties(s: String) -> Result<Vec<IOStatus>> {
    let tokens = get_tokens(s)?;
    Ok(tokens)
}

fn main() {
    env_logger::init();
    execute_main(|| -> Result<()> {
        let args = Args::parse();
        let serialized_context = fs::read_to_string(&args.context_path)?;
        let json_inputs = fs::read_to_string(&args.inputs_path)?;
        let input_parties = parse_input_parties(args.input_parties)?;

        let (tx01, rx01): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (tx12, rx12): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (tx20, rx20): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (tx02, rx02): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (tx10, rx10): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (tx21, rx21): (Sender<String>, Receiver<String>) = mpsc::channel();
        let channels = [
            Channels::new(tx01, rx10, tx02, rx20),
            Channels::new(tx12, rx21, tx10, rx01),
            Channels::new(tx20, rx02, tx21, rx12),
        ];

        let mut children = Vec::new();
        for (id, channels) in channels.into_iter().enumerate() {
            let id = id as u64;
            let ctx = serde_json::from_str::<Context>(&serialized_context)?;
            let mut inputs = serde_json::from_str::<Vec<TypedValue>>(&json_inputs)?;
            assert_eq!(inputs.len(), input_parties.len());
            for i in 0..inputs.len() {
                if let IOStatus::Party(pid) = &input_parties[i] {
                    if *pid != id {
                        eprintln!("Overriding input {i} with zeroes for party {id}");
                        let t = inputs[i].t.clone();
                        inputs[i].value = Value::zero_of_type(t);
                    }
                }
            }
            let child = thread::spawn(move || {
                let evaluator = Evaluator::new(id, channels, None).unwrap();
                let result = get_evaluator_result(ctx, inputs, false, evaluator).unwrap();
                println!(
                    "Party {}: got output {}",
                    id,
                    serde_json::to_string(&result).unwrap()
                );
            });
            children.push(child);
        }
        assert_eq!(
            children
                .into_iter()
                .map(|child| child.join().unwrap())
                .count(),
            3
        );
        Ok(())
    });
}
