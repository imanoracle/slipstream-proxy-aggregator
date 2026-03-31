# Slipstream Multipath Aggregator 🚀

[![Rust](https://img.shields.io/badge/Rust-1.74+-orange.svg)](https://www.rust-lang.org/)
[![Status](https://img.shields.io/badge/Status-Optimized-brightgreen.svg)]()
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

A micro, hyper-optimized asynchronous Load Balancer and Multiplexer built natively in Rust using Tokio. Designed specifically for heavily censored networks (like Iran DPI firewalls), this tool wraps single-threaded VPN/proxy clients (e.g., DNS tunnels) and gracefully distributes enormous payloads across multiple simultaneous proxy subprocesses.

By intentionally instantiating identical parallel tunnels across specifically mapped ISPs (e.g., OpenDNS, Level3), you can bypass arbitrary UDP bandwidth logic and multiply your total underlying tunnel speed natively.

---

## 🔥 Key Features

- **Asynchronous Round-Robin Proxy:** Efficiently binds a global frontend TCP wrapper (e.g., `127.0.0.1:9080`) over an array of dynamically spawned child sockets.
- **Microscopic Footprint:** Configured via ultra-strict Link-Time Optimization (LTO) and raw stripped debug-panics to compile an enterprise engine into a highly executable form factor (`<600KB`).
- **DPI Burst-Evasion:** Injects microscopic 500ms startup jitter organically during thread-cloning to seamlessly weave authentication packets beneath strict ISP Simultaneous Connection DDoS thresholds.
- **Live Terminal Dashboard:** Automatically parses and monitors underlying metrics (Bytes Transferred, Health, Conection Status) using an active Exponential Moving Average (EMA) algorithm exactly like an industrial backend router.
- **Violent Auto-Healing:** Specifically designed for incredibly unstable cellular networks. If external DNS strings reset, it actively executes the trailing process asynchronously, waits 5 seconds to cool down the firewall's DPI engine, and seamlessly reincarnates the child connection `✅ ACTIVE`.

---

## 🛠 Compilation & Deployment

Compile directly from the official repository. We aggressively recommend passing `--release` to leverage the custom macro LTO stripping optimizations configured inside `Cargo.toml`.

```bash
git clone https://github.com/your-username/slipstream-proxy-aggregator
cd slipstream-proxy-aggregator
cargo build --release
```

**To boot the dynamic Dashboard structure:**

```bash
./target/release/slipstream-proxy-aggregator \
  --client-bin /path/to/my-proxy-executable \
  --resolver-file custom_resolvers.txt \
  --domain my_remote_endpoint.xyz \
  -l 9080
```

*(You can intentionally declare the exact identical ISP inside `custom_resolvers.txt` multiple times on sequential lines. The Aggregator does not blindly deduplicate lines. If you paste `8.8.8.8` exactly 5 times, it mathematically drives precisely 5 physical SOCKS nodes right into Google's core server array dynamically.)*

---

## ⚠️ Known Limitations & Shortcomings

While this framework produces unparalleled extraction speed in fully restricted systems, the underlying protocol physics of multiplexed DNS extraction possess rigid boundaries:

1. **The "Connection Burst" Overflow Vulnerability:**
   This aggregator mathematically targets heavy, rigid data downloads (Multi-Segment files via IDM, `curl` bulk fetches, automated scraping payloads). 
   However, modern visual web browsers (such as Firefox/Chrome) aggressively execute incredibly messy HTTP architectures. If you casually type a modern website like `cnn.com`, Firefox inherently triggers **30+ concurrent background micro-connections** at the exact identical millisecond (prefetching Javascript, webfonts, CSS, telemetry).
   
   If Firefox floods the frontend 9080 interface natively with 30 concurrent logic requests all inside 1 second, the backend UDP engine intrinsically drops the excess packets, flagging `❌ ERR/DROP` across all loaded threads recursively. For visual casual browsing, this architecture will occasionally natively reset.

2. **OS File-Descriptor Clamping:**
   Because this physically clones subprocess engines iteratively natively, booting more than ~20 to 30 resolvers globally across standard MacOS/Linux networks might trigger traditional unprivileged file descriptor logic limits natively (`Too many open files`). Keep the resolver list extremely strict to the most reliable 4-8 physical routes realistically matching your backend processing limits.
