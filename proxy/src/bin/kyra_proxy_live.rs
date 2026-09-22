use kyra_proxy::{ObservationConfig, ObservationPipeline, ObserverFactory, Proxy, ProxyConfig};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let listen: SocketAddr = args.next().ok_or("missing listen address")?.parse()?;
    let upstream_addr = args.next().ok_or("missing upstream address")?;
    let upstream_host = args.next().ok_or("missing upstream host")?;
    let observe = args.any(|arg| arg == "--observe");

    let mut config = ProxyConfig::new(listen, upstream_addr);
    config.upstream_host = Some(upstream_host);
    if observe {
        config.observer_factory = Some(observer_factory());
    }
    Proxy::bind(config).await?.serve().await?;
    Ok(())
}

fn observer_factory() -> ObserverFactory {
    Arc::new(|protocol_version| {
        let pipeline =
            ObservationPipeline::for_protocol(protocol_version, 4096, ObservationConfig::default())
                .ok()?;
        let (observer, mut events, _) = pipeline.into_parts();
        tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                match event {
                    kyra_proxy::DecoderEvent::PlayerInfo {
                        added_names,
                        removed_names,
                        ..
                    } => {
                        for username in added_names {
                            eprintln!(
                                "[kyra-live-observe] protocol={protocol_version} player-join {username}"
                            );
                        }
                        for username in removed_names {
                            eprintln!(
                                "[kyra-live-observe] protocol={protocol_version} player-quit {username}"
                            );
                        }
                    }
                    kyra_proxy::DecoderEvent::PlayerRemove { names, .. } => {
                        for username in names {
                            eprintln!(
                                "[kyra-live-observe] protocol={protocol_version} player-quit {username}"
                            );
                        }
                    }
                    kyra_proxy::DecoderEvent::DecodeFailure { error, .. }
                    | kyra_proxy::DecoderEvent::Malformed { error, .. } => {
                        eprintln!(
                            "[kyra-live-observe] protocol={protocol_version} decode-error {error}"
                        );
                    }
                    kyra_proxy::DecoderEvent::Teams { teams, .. } => {
                        let teams_with_players =
                            teams.iter().filter(|team| !team.players.is_empty()).count();
                        eprintln!(
                            "[kyra-live-observe] protocol={protocol_version} teams={} teams_with_players={teams_with_players}",
                            teams.len(),
                        );
                    }
                    kyra_proxy::DecoderEvent::Unknown { .. } => {}
                }
            }
        });
        Some(observer)
    })
}
