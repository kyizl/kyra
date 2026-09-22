use kyra_proto_spike::connection;
use kyra_proto_spike::connection::ConnectionConfig;

fn print_usage_and_exit() -> ! {
    eprintln!(
        "usage: kyra_proto_spike --host <host> --port <port> --protocol-version <n> [--server-address <host>] [--username <name>] [--status]"
    );
    std::process::exit(2);
}

fn parse_args() -> (ConnectionConfig, bool) {
    let mut host: Option<String> = None;
    let mut server_address: Option<String> = None;
    let mut port: u16 = 25565;
    let mut username: String = "KyraSpike".to_string();
    let mut protocol_version: Option<i32> = None;
    let mut status = false;

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--host" => host = args.next(),
            "--server-address" => server_address = args.next(),
            "--port" => {
                port = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| print_usage_and_exit());
            }
            "--username" => {
                username = args.next().unwrap_or_else(|| print_usage_and_exit());
            }
            "--protocol-version" => {
                protocol_version = args.next().and_then(|s| s.parse().ok());
            }
            "--status" => status = true,
            _ => print_usage_and_exit(),
        }
    }

    let (Some(host), Some(protocol_version)) = (host, protocol_version) else {
        print_usage_and_exit();
    };

    (
        ConnectionConfig {
            host,
            server_address,
            port,
            username,
            protocol_version,
        },
        status,
    )
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let (config, status) = parse_args();

    let result = if status {
        connection::status(&config).await
    } else {
        connection::run(config).await
    };
    if let Err(err) = result {
        tracing::error!(%err, "connection failed");
        std::process::exit(1);
    }
}
