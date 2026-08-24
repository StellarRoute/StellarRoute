//! Opt-in read-only Stellar CCTP strict simulate→assemble probe (public Testnet RPC).
//!
//! Usage:
//!   cargo run -p stellarroute-api --bin cctp-live-readiness-probe
//!   SIMULATE_SOURCE=G... cargo run -p stellarroute-api --bin cctp-live-readiness-probe

use stellarroute_api::cctp::config::CctpConfig;
use stellarroute_api::cctp::stellar_builder_simulation::probe_strict_simulation_assembly_with_evidence;
use stellarroute_api::cctp::stellar_readiness_probes::probe_stellar_contracts;
use stellarroute_api::cctp::stellar_rpc::StellarRpcClient;

#[tokio::main]
async fn main() {
    let config = CctpConfig::default_testnet();
    let source = std::env::var("SIMULATE_SOURCE").ok();
    let rpc = match StellarRpcClient::new(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{{\"error\":\"rpc client: {e}\"}}");
            std::process::exit(1);
        }
    };
    let contract_probe = probe_stellar_contracts(&config).await;
    match probe_strict_simulation_assembly_with_evidence(&rpc, &config, source.as_deref()).await {
        Ok(evidence) => {
            let out = serde_json::json!({
                "contract_probes_ok": contract_probe.all_ok(),
                "contract_probes": {
                    "rpc_ok": contract_probe.rpc_ok,
                    "message_transmitter_ok": contract_probe.message_transmitter_ok,
                    "forwarder_ok": contract_probe.forwarder_ok,
                    "token_messenger_ok": contract_probe.token_messenger_ok,
                    "usdc_ok": contract_probe.usdc_ok,
                },
                "strict_simulation": evidence,
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
        Err(err) => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "contract_probes_ok": contract_probe.all_ok(),
                    "contract_probes": {
                        "rpc_ok": contract_probe.rpc_ok,
                        "message_transmitter_ok": contract_probe.message_transmitter_ok,
                        "forwarder_ok": contract_probe.forwarder_ok,
                        "token_messenger_ok": contract_probe.token_messenger_ok,
                        "usdc_ok": contract_probe.usdc_ok,
                    },
                    "strict_simulation_error": err,
                }))
                .unwrap()
            );
            std::process::exit(1);
        }
    }
}
