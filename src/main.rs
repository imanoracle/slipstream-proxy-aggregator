use clap::Parser;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::sleep;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Slipstream Multipath Aggregator")]
struct Args {
    #[arg(long, default_value = "./target/release/slipstream-client")]
    client_bin: PathBuf,

    #[arg(short = 'r', long)]
    resolver: Option<String>,

    #[arg(long)]
    resolver_file: Option<PathBuf>,

    #[arg(long, default_value_t = 1)]
    instances: usize,

    #[arg(short = 'd', long)]
    domain: String,

    #[arg(short = 'l', long, default_value_t = 9000)]
    tcp_listen_port: u16,
}

#[derive(Debug, Clone)]
struct InstanceState {
    id: usize,
    port: u16,
    ip: String,
    status: &'static str,
    ready: bool,
    conns: usize,
    total_bytes: u64,
    last_bytes: u64,
    current_speed: f64,
    ema_speed: f64,
}

impl InstanceState {
    fn score(&self) -> f64 {
        let speed_norm = self.ema_speed.max(1.0);
        (0.2 * self.conns as f64) - (0.8 * (speed_norm / 1024.0))
    }
}

async fn forward(
    mut reader: tokio::net::tcp::ReadHalf<'_>,
    mut writer: tokio::net::tcp::WriteHalf<'_>,
    state_lock: Arc<RwLock<Vec<InstanceState>>>,
    idx: usize,
    chunk_size: usize,
) {
    let mut buf = vec![0; chunk_size];
    loop {
        match reader.read(&mut buf).await {
            Ok(0) => break, // EOF
            Ok(n) => {
                if writer.write_all(&buf[..n]).await.is_err() {
                    break;
                }
                let mut guard = state_lock.write().await;
                guard[idx].total_bytes += n as u64;
            }
            Err(_) => break,
        }
    }
}

async fn handle_tunneling(mut client: TcpStream, states: Arc<RwLock<Vec<InstanceState>>>) {
    let best_idx = {
        let guard = states.read().await;
        let best = guard
            .iter()
            .enumerate()
            .filter(|(_, inst)| inst.ready && inst.conns < 10)
            .min_by(|(_, a), (_, b)| {
                a.score().partial_cmp(&b.score()).unwrap_or(std::cmp::Ordering::Equal)
            });

        match best {
            Some((idx, _)) => idx,
            None => return, // No ready instances available
        }
    };

    {
        let mut guard = states.write().await;
        guard[best_idx].conns += 1;
    }

    let port = { states.read().await[best_idx].port };
    if let Ok(mut remote) = tokio::time::timeout(
        Duration::from_secs(5),
        TcpStream::connect(("127.0.0.1", port)),
    )
    .await
    .unwrap_or(Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "Timeout")))
    {
        let (cr, cw) = client.split();
        let (rr, rw) = remote.split();

        let up = forward(cr, rw, states.clone(), best_idx, 4096);
        let down = forward(rr, cw, states.clone(), best_idx, 262144);
        tokio::select! {
            _ = up => {}
            _ = down => {}
        }
    }

    {
        let mut guard = states.write().await;
        guard[best_idx].conns = guard[best_idx].conns.saturating_sub(1);
    }
}

async fn spawn_instance(
    idx: usize,
    port: u16,
    ip: String,
    args: Args,
    states: Arc<RwLock<Vec<InstanceState>>>,
) {
    loop {
        {
            let mut guard = states.write().await;
            guard[idx].status = "Connecting";
            guard[idx].ready = false;
        }

        let mut child = Command::new(&args.client_bin)
            .arg("--domain")
            .arg(&args.domain)
            .arg("--tcp-listen-port")
            .arg(port.to_string())
            .arg("--resolver")
            .arg(format!("{}:53", ip))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn client process");

        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout).lines();

        while let Ok(Some(line)) = reader.next_line().await {
            if line.contains("Connection ready") {
                let mut guard = states.write().await;
                guard[idx].status = "✅ ACTIVE";
                guard[idx].ready = true;
            } else if line.contains("1044") || line.contains("unavailable") || line.contains("reset event") {
                let mut guard = states.write().await;
                guard[idx].status = "❌ ERR/DROP";
                guard[idx].ready = false;
                
                // CRITICAL FIX: Kill the stalled connection so the loop automatically restarts it in 5 seconds!
                let _ = child.kill().await;
                break;
            }
        }

        let _ = child.wait().await;
        {
            let mut guard = states.write().await;
            guard[idx].status = "💀 DEAD";
            guard[idx].ready = false;
        }

        sleep(Duration::from_secs(5)).await;
    }
}

async fn dashboard_loop(states: Arc<RwLock<Vec<InstanceState>>>, lb_port: u16) {
    loop {
        sleep(Duration::from_secs(2)).await;

        print!("\x1B[H\x1B[J"); // clear screen
        println!("🚀 SLIPSTREAM RUST AGGREGATOR | LB: {}", lb_port);

        let mut ready_count = 0;
        let mut total_speed = 0.0;
        let mut total_data = 0;

        {
            let mut guard = states.write().await;
            for i in guard.iter_mut() {
                if i.status != "💀 DEAD" && i.status != "❌ ERR/DROP" {
                    i.current_speed = (i.total_bytes.saturating_sub(i.last_bytes) as f64) / 2.0;
                    i.last_bytes = i.total_bytes;
                    i.ema_speed = (i.current_speed * 0.3) + (i.ema_speed * 0.7);
                }

                if i.ready {
                    ready_count += 1;
                }
                total_speed += i.current_speed;
                total_data += i.total_bytes;
            }

            println!(
                "✅ Active: {}/{} | 📈 {:>6.2} KB/s | 📦 Session Total: {:>6.1} MB",
                ready_count,
                guard.len(),
                total_speed / 1024.0,
                total_data as f64 / 1048576.0
            );
            println!("{:-<100}", "");
            println!("{:<6} {:<16} {:<14} {:<6} {:<12} {:<10} SCORE", "PORT", "DNS IP", "STATUS", "CONNS", "EMA SPD", "TOTAL");
            println!("{:-<100}", "");

            for i in guard.iter() {
                println!(
                    "{:<6} {:<16} {:<14} {:<6} {:>6.1} KB/s {:>6.1} MB [{:>6.1}]",
                    i.port,
                    i.ip,
                    i.status,
                    i.conns,
                    i.ema_speed / 1024.0,
                    i.total_bytes as f64 / 1048576.0,
                    i.score()
                );
            }
            println!("{:-<100}", "");
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut ips = Vec::new();

    if let Some(r) = &args.resolver {
        ips.push(r.clone());
    }
    if let Some(path) = &args.resolver_file {
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let text = line.trim();
                if !text.is_empty() {
                    ips.push(text.to_string());
                }
            }
        }
    }

    let mut final_ips = Vec::new();
    for ip in ips {
        for _ in 0..args.instances {
            final_ips.push(ip.clone());
        }
    }

    if final_ips.is_empty() {
        eprintln!("Error: No resolvers provided!");
        return;
    }

    let mut initial_states = Vec::new();
    let mut current_port = 11000;

    for (i, ip) in final_ips.iter().enumerate() {
        initial_states.push(InstanceState {
            id: i,
            port: current_port,
            ip: ip.clone(),
            status: "Init",
            ready: false,
            conns: 0,
            total_bytes: 0,
            last_bytes: 0,
            current_speed: 0.0,
            ema_speed: 0.0,
        });
        current_port += 1;
    }

    let states = Arc::new(RwLock::new(initial_states));

    for i in 0..final_ips.len() {
        let ip = final_ips[i].clone();
        let args_clone = args.clone();
        let states_clone = states.clone();
        let port = 11000 + (i as u16);
        tokio::spawn(spawn_instance(i, port, ip, args_clone, states_clone));
        // Securely stagger connections natively so endpoints like OpenDNS don't rate-limit simultaneous hits
        sleep(Duration::from_millis(500)).await;
    }

    let dashboard_states = states.clone();
    let lb_port = args.tcp_listen_port;
    tokio::spawn(dashboard_loop(dashboard_states, lb_port));

    let listener = TcpListener::bind(("0.0.0.0", lb_port)).await.unwrap();
    loop {
        if let Ok((client, _)) = listener.accept().await {
            let states_clone = states.clone();
            tokio::spawn(handle_tunneling(client, states_clone));
        }
    }
}
