use std::{
    env,
    io::Write,
    net::{SocketAddr, TcpStream},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
extern crate rand;

fn setup() -> Result<SocketAddr, u32> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("usage: tcp-loadtest <ip> <port>");
        println!("example: tcp-loadtest 127.0.0.1 52000");
        return Err(1);
    }
    let a: SocketAddr = format!("{}:{}", args[1], args[2]).parse().unwrap();
    return Ok(a);
}

fn main() -> Result<(), u32> {
    let addr = Arc::new(setup()?);
    let data: Arc<String> = Arc::new(
        (0..1000000)
            .map(|_| (0x20u8 + (rand::random::<f32>() * 96.0) as u8) as char)
            .collect(),
    );

    let total_bytes_send = Arc::new(AtomicU64::new(0));
    let total_send_count = Arc::new(AtomicU64::new(0));
    let total_finished_threads = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();

    let mut threads = Vec::new();
    let thread_count = 4;
    for i in 0..thread_count {
        let addr = addr.clone();
        let total_bytes_send = total_bytes_send.clone();
        let total_send_count = total_send_count.clone();
        let total_finished_threads = total_finished_threads.clone();
        let data = data.clone();
        threads.push(thread::spawn(move || {
            let mut stream = match TcpStream::connect(addr.as_ref()) {
                Err(err) => {
                    total_finished_threads.fetch_add(1, Ordering::Relaxed);
                    panic!("could not open connection: {}", err);
                },
                Ok(s) => s,
            };
            let mut thread_bytes_send: u64 = 0;
            let mut thread_last_sync = Instant::now();

            loop {
                let tosend: u64 = rand::random::<u16>() as u64 * 10 + 1000;
                thread_bytes_send += tosend;

                let cur = Instant::now();
                if cur.duration_since(thread_last_sync) >= Duration::from_secs(1) {
                    thread_last_sync = cur;
                    println!("[{}]send: {}MiB/s", i, thread_bytes_send / (1024 * 1024));
                    total_bytes_send.fetch_add(thread_bytes_send, Ordering::Relaxed);
                    thread_bytes_send = 0;
                }

                total_send_count.fetch_add(1, Ordering::Relaxed);
                let ret = stream.write_all(data[0..(tosend as usize)].as_bytes());
                if ret.is_err() {
                    println!("[{}] error: {}", i, ret.err().unwrap());
                    total_finished_threads.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                //stream.flush();
            }
        }));
    }

    while total_finished_threads.load(Ordering::Relaxed) < thread_count {
        thread::sleep(Duration::from_millis(10));
    }

    let cur = Instant::now();
    let dur = cur.duration_since(start_time);
    println!("================");
    println!("test endet");
    println!(
        "total send: {}MiB",
        total_bytes_send.load(Ordering::Relaxed) / (1024 * 1024)
    );
    println!("total time: {}s", dur.as_secs());
    println!(
        "average: {}KiB/s",
        total_bytes_send.load(Ordering::Relaxed) * 1000 / dur.as_millis() as u64 / 1024
    );
    println!(
        "send count: {}/s",
        total_send_count.load(Ordering::Relaxed) * 1000 / dur.as_millis() as u64
    );

    Ok(())
}
