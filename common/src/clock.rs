use std::time::{Duration, SystemTime};

pub struct Clock {
    time: SystemTime,
    tps_counter: u64,
    last_tps_time: SystemTime,
    last_tps: f64,
}

impl Clock {
    pub fn new() -> Clock {
        Clock {
            time: SystemTime::now(),
            tps_counter: 0,
            last_tps_time: SystemTime::now(),
            last_tps: 1.0,
        }
    }

    // returns delta and timestamp
    pub fn delta(&self) -> (Duration, SystemTime) {
        let cur = SystemTime::now();
        let delta = cur.duration_since(self.time);
        (delta.unwrap(), cur)
    }

    pub fn tick(&mut self, tick_time: Duration) {

        let delta = self.delta();
        // sleep is only necessary if we are fast enough
        //println!("delta {:?} tick_time {:?}", delta, tick_time);
        if delta.0 < tick_time {
            let sleep_time = tick_time - delta.0;
            //thread::sleep(sleep_time);
        } else {
            warn!("clock is running behind");
        }
        self.time = SystemTime::now();
        //calculate tps
        self.tps_counter += 1;
        if self.last_tps_time + Duration::from_millis(5000) < self.time {
            let tps_duration = self.time.duration_since(self.last_tps_time).unwrap();
            // get real time that happen scince last tps, not only 5000 ms
            let seconds: f64 = tps_duration.as_secs() as f64 + tps_duration.subsec_micros() as f64 / 1000000.0;
            self.last_tps = self.tps_counter as f64 / seconds;
            self.tps_counter = 0;
            self.last_tps_time = self.time;
            info!("tps: {}", self.last_tps);
        };
    }

    pub fn last_tps(&self) -> f64 {
        self.last_tps
    }
}
