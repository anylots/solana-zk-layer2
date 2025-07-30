use crate::node::Node;
use log::info;

mod batcher;
mod executor;
mod node;
mod rpc;
mod validator;

#[tokio::main]
async fn main() {
    // Step1. init log sys
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Starting solana layer2 sequencer...");

    // Step2. Start sequencer node
    tokio::spawn(async {
        let mut sequencer_node = Node::new().await.expect("Init sequencer node failed");
        sequencer_node.start().await
    });

    // Step3. Start rpc server
    rpc::start().await;
}
