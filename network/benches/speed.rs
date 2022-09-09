use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::{net::SocketAddr, sync::Arc};
use tokio::{runtime::Runtime, sync::Mutex};
use veloren_network::{
    ConnectAddr, ListenAddr, Message, Network, Participant, Pid, Promises, Stream,
};

fn serialize(data: &[u8], stream: &Stream) { let _ = Message::serialize(data, stream.params()); }

async fn stream_msg(s1_a: Arc<Mutex<Stream>>, s1_b: Arc<Mutex<Stream>>, data: &[u8], cnt: usize) {
    let mut s1_b = s1_b.lock().await;
    let m = Message::serialize(&data, s1_b.params());
    std::thread::spawn(move || {
        let s1_a = s1_a.try_lock().unwrap();
        for _ in 0..cnt {
            s1_a.send_raw(&m).unwrap();
        }
    });
    for _ in 0..cnt {
        s1_b.recv_raw().await.unwrap();
    }
}

fn rt() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
}

fn criterion_util(c: &mut Criterion) {
    let mut c = c.benchmark_group("net_util");
    c.significance_level(0.1).sample_size(100);

    let (r, _n_a, p_a, s1_a, _n_b, _p_b, _s1_b) =
        network_participant_stream((ListenAddr::Mpsc(5000), ConnectAddr::Mpsc(5000)));
    let s2_a = r.block_on(p_a.open(4, Promises::COMPRESSED, 0)).unwrap();

    c.throughput(Throughput::Bytes(1000))
        .bench_function("message_serialize", |b| {
            let data = vec![0u8; 1000];
            b.iter(|| serialize(&data, &s1_a))
        });
    c.throughput(Throughput::Bytes(1000))
        .bench_function("message_serialize_compress", |b| {
            let data = vec![0u8; 1000];
            b.iter(|| serialize(&data, &s2_a))
        });
}

fn criterion_mpsc(c: &mut Criterion) {
    let mut c = c.benchmark_group("net_mpsc");
    c.significance_level(0.1).sample_size(10);

    let (_r, _n_a, _p_a, s1_a, _n_b, _p_b, s1_b) =
        network_participant_stream((ListenAddr::Mpsc(5000), ConnectAddr::Mpsc(5000)));
    let s1_a = Arc::new(Mutex::new(s1_a));
    let s1_b = Arc::new(Mutex::new(s1_b));

    c.throughput(Throughput::Bytes(100000000)).bench_function(
        BenchmarkId::new("100MB_in_10000_msg", ""),
        |b| {
            let data = vec![155u8; 100_000];
            b.to_async(rt()).iter_with_setup(
                || (Arc::clone(&s1_a), Arc::clone(&s1_b)),
                |(s1_a, s1_b)| stream_msg(s1_a, s1_b, &data, 1_000),
            )
        },
    );
    c.throughput(Throughput::Elements(100000)).bench_function(
        BenchmarkId::new("100000_tiny_msg", ""),
        |b| {
            let data = vec![3u8; 5];
            b.to_async(rt()).iter_with_setup(
                || (Arc::clone(&s1_a), Arc::clone(&s1_b)),
                |(s1_a, s1_b)| stream_msg(s1_a, s1_b, &data, 100_000),
            )
        },
    );
    c.finish();
    drop((_n_a, _p_a, _n_b, _p_b));
}

fn criterion_tcp(c: &mut Criterion) {
    let mut c = c.benchmark_group("net_tcp");
    c.significance_level(0.1).sample_size(10);

    let socket_addr = SocketAddr::from(([127, 0, 0, 1], 5000));
    let (_r, _n_a, _p_a, s1_a, _n_b, _p_b, s1_b) =
        network_participant_stream((ListenAddr::Tcp(socket_addr), ConnectAddr::Tcp(socket_addr)));
    let s1_a = Arc::new(Mutex::new(s1_a));
    let s1_b = Arc::new(Mutex::new(s1_b));

    c.throughput(Throughput::Bytes(100000000)).bench_function(
        BenchmarkId::new("100MB_in_1000_msg", ""),
        |b| {
            let data = vec![155u8; 100_000];
            b.to_async(rt()).iter_with_setup(
                || (Arc::clone(&s1_a), Arc::clone(&s1_b)),
                |(s1_a, s1_b)| stream_msg(s1_a, s1_b, &data, 1_000),
            )
        },
    );
    c.throughput(Throughput::Elements(100000)).bench_function(
        BenchmarkId::new("100000_tiny_msg", ""),
        |b| {
            let data = vec![3u8; 5];
            b.to_async(rt()).iter_with_setup(
                || (Arc::clone(&s1_a), Arc::clone(&s1_b)),
                |(s1_a, s1_b)| stream_msg(s1_a, s1_b, &data, 100_000),
            )
        },
    );
    c.finish();
    drop((_n_a, _p_a, _n_b, _p_b));
}

criterion_group!(benches, criterion_util, criterion_mpsc, criterion_tcp);
criterion_main!(benches);

pub fn network_participant_stream(
    addr: (ListenAddr, ConnectAddr),
) -> (
    Runtime,
    Network,
    Participant,
    Stream,
    Network,
    Participant,
    Stream,
) {
    let runtime = Runtime::new().unwrap();
    let (n_a, p1_a, s1_a, n_b, p1_b, s1_b) = runtime.block_on(async {
        let mut n_a = Network::new(Pid::fake(0), &runtime);
        let n_b = Network::new(Pid::fake(1), &runtime);

        n_a.listen(addr.0).await.unwrap();
        let mut p1_b = n_b.connect(addr.1).await.unwrap();
        let p1_a = n_a.connected().await.unwrap();

        let s1_a = p1_a.open(4, Promises::empty(), 0).await.unwrap();
        let s1_b = p1_b.opened().await.unwrap();

        (n_a, p1_a, s1_a, n_b, p1_b, s1_b)
    });
    (runtime, n_a, p1_a, s1_a, n_b, p1_b, s1_b)
}
