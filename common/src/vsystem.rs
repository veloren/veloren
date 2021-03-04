use crate::metrics::SysMetrics;
use specs::{ReadExpect, RunNow, System};
use std::{collections::HashMap, time::Instant};

/// measuring the level of threads a unit of code ran on. Use Rayon when it ran
/// on their threadpool. Use Exact when you know on how many threads your code
/// ran on exactly.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ParMode {
    None, /* Job is not running at all */
    Single,
    Rayon,
    Exact(u16),
}

//TODO: make use of the phase of a system for advanced scheduling and logging
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    Create,
    Review,
    Apply,
}

//TODO: make use of the origin of the system for better logging
#[derive(Clone, PartialEq, Debug)]
pub enum Origin {
    Common,
    Client,
    Server,
    Frontend(&'static str),
}

#[derive(Default, Debug, Clone)]
pub struct CpuTimeline {
    /// 
    /// - The first entry will always be ParMode::Single, as when the
    ///   System::run is executed, we run
    /// single threaded until we start a Rayon::ParIter or similar
    /// - The last entry will contain the end time of the System. To mark the
    ///   End it will always contain
    /// ParMode::None, which means from that point on 0 CPU threads work in this
    /// system
    measures: Vec<(Instant, ParMode)>,
}

#[derive(Default)]
pub struct CpuTimeStats {
    /// the first entry will always be 0, the last entry will always be `dt`
    /// `usage` starting from `ns`
    measures: Vec<(/* ns */ u64, /* usage */ f32)>,
}

/// Parallel Mode tells us how much you are scaling. `None` means your code
/// isn't running. `Single` means you are running single threaded.
/// `Rayon` means you are running on the rayon threadpool.
impl ParMode {
    fn threads(&self, rayon_threads: u16) -> u16 {
        match self {
            ParMode::None => 0,
            ParMode::Single => 1,
            ParMode::Rayon => rayon_threads,
            ParMode::Exact(u) => *u,
        }
    }
}

impl CpuTimeline {
    fn reset(&mut self) {
        self.measures.clear();
        self.measures.push((Instant::now(), ParMode::Single));
    }

    /// Start a new measurement. par will be covering the parallelisation AFTER
    /// this statement, till the next / end of the VSystem.
    pub fn measure(&mut self, par: ParMode) { self.measures.push((Instant::now(), par)); }

    fn end(&mut self) { self.measures.push((Instant::now(), ParMode::None)); }

    fn get(&self, time: Instant) -> ParMode {
        match self.measures.binary_search_by_key(&time, |&(a, _)| a) {
            Ok(id) => self.measures[id].1,
            Err(0) => ParMode::None, /* not yet started */
            Err(id) => self.measures[id - 1].1,
        }
    }
}

impl CpuTimeStats {
    pub fn length_ns(&self) -> u64 { self.end_ns() - self.start_ns() }

    pub fn start_ns(&self) -> u64 {
        self.measures
            .iter()
            .find(|e| e.1 > 0.001)
            .unwrap_or(&(0, 0.0))
            .0
    }

    pub fn end_ns(&self) -> u64 { self.measures.last().unwrap_or(&(0, 0.0)).0 }

    pub fn avg_threads(&self) -> f32 {
        let mut sum = 0.0;
        for w in self.measures.windows(2) {
            let len = w[1].0 - w[0].0;
            let h = w[0].1;
            sum += len as f32 * h;
        }
        sum / (self.length_ns() as f32)
    }
}

/// The Idea is to transform individual timelines per system to a map of all
/// cores and what they (prob) are working on.
///
/// # Example
///
/// - Input: 3 services, 0 and 1 are 100% parallel and 2 is single threaded. `-`
///   means no work for *0.5s*. `#` means full work for *0.5s*. We see the first
///   service starts after 1s and runs for 3s The second one starts a sec later
///   and runs for 4s. The last service runs 2.5s after the tick start and runs
///   for 1s. Read left to right.
/// ```ignore
/// [--######------]
/// [----########--]
/// [-----##-------]
/// ```
///
/// - Output: a Map that calculates where our 6 cores are spending their time.
///   Here each number means 50% of a core is working on it. A '-' represents an
///   idling core. We start with all 6 cores idling. Then all cores start to
///   work on task 0. 2s in, task1 starts and we have to split cores. 2.5s in
///   task2 starts. We have 6 physical threads but work to fill 13. Later task 2
///   and task 0 will finish their work and give more threads for task 1 to work
///   on. Read top to bottom
/// ```ignore
/// 0-1s     [------------]
/// 1-2s     [000000000000]
/// 2-2.5s   [000000111111]
/// 2.5-3.5s [000001111122]
/// 3.5-4s   [000000111111]
/// 4-6s     [111111111111]
/// 6s..     [------------]
/// ```
pub fn gen_stats(
    timelines: &HashMap<String, CpuTimeline>,
    tick_work_start: Instant,
    rayon_threads: u16,
    physical_threads: u16,
) -> HashMap<String, CpuTimeStats> {
    let mut result = HashMap::new();
    let mut all = timelines
        .iter()
        .map(|(s, t)| {
            let mut stat = CpuTimeStats::default();
            stat.measures.push((0, 0.0));
            result.insert(s.clone(), stat);
            t.measures.iter().map(|e| &e.0)
        })
        .flatten()
        .collect::<Vec<_>>();

    all.sort();
    all.dedup();
    for time in all {
        let relative_time = time.duration_since(tick_work_start).as_nanos() as u64;
        // get all parallelisation at this particular time
        let individual_cores_wanted = timelines
            .iter()
            .map(|(k, t)| (k, t.get(*time).threads(rayon_threads)))
            .collect::<Vec<_>>();
        let total = individual_cores_wanted
            .iter()
            .map(|(_, a)| a)
            .sum::<u16>()
            .max(1) as f32;
        let total_or_max = total.max(physical_threads as f32);
        // update ALL states
        for individual in individual_cores_wanted.iter() {
            let actual = (individual.1 as f32 / total_or_max) * physical_threads as f32;
            let p = result.get_mut(individual.0).unwrap();
            if (p.measures.last().unwrap().1 - actual).abs() > 0.0001 {
                p.measures.push((relative_time, actual));
            }
        }
    }
    result
}

/// This trait wraps around specs::System and does additional veloren tasks like
/// metrics collection
///
/// ```
/// use specs::Read;
/// pub use veloren_common::vsystem::{Origin, ParMode, Phase, VJob, VSystem};
/// # use std::time::Duration;
/// pub struct Sys;
/// impl<'a> VSystem<'a> for Sys {
///     type SystemData = (Read<'a, ()>, Read<'a, ()>);
///
///     const NAME: &'static str = "example";
///     const ORIGIN: Origin = Origin::Frontend("voxygen");
///     const PHASE: Phase = Phase::Create;
///
///     fn run(job: &mut VJob<Self>, (_read, _read2): Self::SystemData) {
///         std::thread::sleep(Duration::from_millis(100));
///         job.cpu_stats.measure(ParMode::Rayon);
///         std::thread::sleep(Duration::from_millis(500));
///         job.cpu_stats.measure(ParMode::Single);
///         std::thread::sleep(Duration::from_millis(40));
///     }
/// }
/// ```
pub trait VSystem<'a> {
    const NAME: &'static str;
    const PHASE: Phase;
    const ORIGIN: Origin;

    type SystemData: specs::SystemData<'a>;
    fn run(job: &mut VJob<Self>, data: Self::SystemData);
    fn sys_name() -> String { format!("{}_sys", Self::NAME) }
}

pub fn dispatch<'a, 'b, T>(builder: &mut specs::DispatcherBuilder<'a, 'b>, dep: &[&str])
where
    T: for<'c> VSystem<'c> + Send + 'a + Default,
{
    builder.add(VJob::<T>::default(), &T::sys_name(), dep);
}

pub fn run_now<'a, 'b, T>(world: &'a specs::World)
where
    T: for<'c> VSystem<'c> + Send + 'a + Default,
{
    VJob::<T>::default().run_now(world);
}

/// This Struct will wrap the System in order to avoid the can only impl trait
/// for local defined structs error It also contains the cpu measurements
pub struct VJob<T>
where
    T: ?Sized,
{
    pub own: Box<T>,
    pub cpu_stats: CpuTimeline,
}

impl<'a, T> System<'a> for VJob<T>
where
    T: VSystem<'a>,
{
    type SystemData = (T::SystemData, ReadExpect<'a, SysMetrics>);

    fn run(&mut self, data: Self::SystemData) {
        crate::span!(_guard, "run", &format!("{}::Sys::run", T::NAME));
        self.cpu_stats.reset();
        T::run(self, data.0);
        self.cpu_stats.end();
        data.1
            .stats
            .lock()
            .unwrap()
            .insert(T::NAME.to_string(), self.cpu_stats.clone());
    }
}

impl<'a, T> Default for VJob<T>
where
    T: VSystem<'a> + Default,
{
    fn default() -> Self {
        Self {
            own: Box::new(T::default()),
            cpu_stats: CpuTimeline::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use float_cmp::approx_eq;
    use std::time::Duration;

    fn mock_timelines(
        tick_start: Instant,
        durations: Vec<(u64, u64, ParMode)>,
    ) -> HashMap<String, CpuTimeline> {
        let job = durations
            .iter()
            .enumerate()
            .map(|(i, (s, e, p))| {
                (
                    i,
                    tick_start + Duration::from_millis(*s),
                    tick_start + Duration::from_millis(*e),
                    *p,
                )
            })
            .collect::<Vec<_>>();

        job.iter()
            .map(|(i, f, s, p)| {
                (i.to_string(), CpuTimeline {
                    measures: vec![(*f, *p), (*s, ParMode::None)],
                })
            })
            .collect()
    }

    #[test]
    fn single() {
        const RAYON_THREADS: u16 = 4;
        const PHYSICAL_THREADS: u16 = RAYON_THREADS;
        let tick_start = Instant::now();
        let job_d = vec![(500, 1500, ParMode::Rayon)];
        let timelines = mock_timelines(tick_start, job_d);

        let stats = gen_stats(&timelines, tick_start, RAYON_THREADS, PHYSICAL_THREADS);

        const THREADS: f32 = PHYSICAL_THREADS as f32;

        let s = &stats["0"];
        let measures = &s.measures;
        assert_eq!(measures.len(), 3);
        assert_eq!(measures[0].0, 0);
        assert!(approx_eq!(f32, measures[0].1, 0.0));
        assert_eq!(measures[1].0, 500000000);
        assert!(approx_eq!(f32, measures[1].1, THREADS));
        assert_eq!(measures[2].0, 1500000000);
        assert!(approx_eq!(f32, measures[2].1, 0.0));
        assert_eq!(s.start_ns(), 500000000);
        assert_eq!(s.end_ns(), 1500000000);
        assert_eq!(s.length_ns(), 1000000000);
        assert!(approx_eq!(f32, s.avg_threads(), THREADS));
    }

    #[test]
    fn two_jobs() {
        const RAYON_THREADS: u16 = 8;
        const PHYSICAL_THREADS: u16 = RAYON_THREADS;
        let tick_start = Instant::now();
        let job_d = vec![(2000, 3000, ParMode::Single), (5000, 6500, ParMode::Single)];
        let timelines = mock_timelines(tick_start, job_d);

        let stats = gen_stats(&timelines, tick_start, RAYON_THREADS, PHYSICAL_THREADS);

        let s = &stats["0"];
        let measures = &s.measures;
        assert_eq!(measures.len(), 3);
        assert_eq!(measures[0].0, 0);
        assert!(approx_eq!(f32, measures[0].1, 0.0));
        assert_eq!(measures[1].0, 2000000000);
        assert!(approx_eq!(f32, measures[1].1, 1.0));
        assert_eq!(measures[2].0, 3000000000);
        assert!(approx_eq!(f32, measures[2].1, 0.0));
        assert_eq!(s.start_ns(), 2000000000);
        assert_eq!(s.end_ns(), 3000000000);
        assert_eq!(s.length_ns(), 1000000000);
        assert!(approx_eq!(f32, s.avg_threads(), 1.0));

        let s = &stats["1"];
        let measures = &s.measures;
        assert_eq!(measures.len(), 3);
        assert_eq!(measures[0].0, 0);
        assert!(approx_eq!(f32, measures[0].1, 0.0));
        assert_eq!(measures[1].0, 5000000000);
        assert!(approx_eq!(f32, measures[1].1, 1.0));
        assert_eq!(measures[2].0, 6500000000);
        assert!(approx_eq!(f32, measures[2].1, 0.0));
        assert_eq!(s.start_ns(), 5000000000);
        assert_eq!(s.end_ns(), 6500000000);
        assert_eq!(s.length_ns(), 1500000000);
        assert!(approx_eq!(f32, s.avg_threads(), 1.0));
    }

    #[test]
    fn generate_stats() {
        const RAYON_THREADS: u16 = 6;
        const PHYSICAL_THREADS: u16 = RAYON_THREADS;
        let tick_start = Instant::now();
        let job_d = vec![
            (2000, 5000, ParMode::Rayon),
            (3000, 7000, ParMode::Rayon),
            (3500, 4500, ParMode::Single),
        ];
        let timelines = mock_timelines(tick_start, job_d);

        let stats = gen_stats(&timelines, tick_start, RAYON_THREADS, PHYSICAL_THREADS);

        const THREADS: f32 = PHYSICAL_THREADS as f32;

        let s = &stats["0"];
        let measures = &s.measures;
        assert_eq!(measures.len(), 6);
        assert_eq!(measures[0].0, 0);
        assert!(approx_eq!(f32, measures[0].1, 0.0));
        assert_eq!(measures[1].0, 2000000000);
        assert!(approx_eq!(f32, measures[1].1, THREADS));
        assert_eq!(measures[2].0, 3000000000);
        assert!(approx_eq!(f32, measures[2].1, THREADS / 2.0));
        assert_eq!(measures[3].0, 3500000000);
        assert!(approx_eq!(
            f32,
            measures[3].1,
            THREADS * THREADS / (THREADS * 2.0 + 1.0)
        ));
        assert_eq!(measures[4].0, 4500000000);
        assert!(approx_eq!(f32, measures[4].1, THREADS / 2.0));
        assert_eq!(measures[5].0, 5000000000);
        assert!(approx_eq!(f32, measures[5].1, 0.0));
        assert_eq!(s.start_ns(), 2000000000);
        assert_eq!(s.end_ns(), 5000000000);
        assert_eq!(s.length_ns(), 3000000000);
        assert!(approx_eq!(f32, s.avg_threads(), 3.923077));

        let s = &stats["1"];
        let measures = &s.measures;
        assert_eq!(measures.len(), 6);
        assert_eq!(measures[0].0, 0);
        assert!(approx_eq!(f32, measures[0].1, 0.0));
        assert_eq!(measures[1].0, 3000000000);
        assert!(approx_eq!(f32, measures[1].1, THREADS / 2.0));
        assert_eq!(measures[2].0, 3500000000);
        assert!(approx_eq!(
            f32,
            measures[2].1,
            THREADS * THREADS / (THREADS * 2.0 + 1.0)
        ));
        assert_eq!(measures[3].0, 4500000000);
        assert!(approx_eq!(f32, measures[3].1, THREADS / 2.0));
        assert_eq!(measures[4].0, 5000000000);
        assert!(approx_eq!(f32, measures[4].1, THREADS));
        assert_eq!(measures[5].0, 7000000000);
        assert!(approx_eq!(f32, measures[5].1, 0.0));
        assert_eq!(s.start_ns(), 3000000000);
        assert_eq!(s.end_ns(), 7000000000);
        assert_eq!(s.length_ns(), 4000000000);
        assert!(approx_eq!(f32, s.avg_threads(), 4.4423075));

        let s = &stats["2"];
        let measures = &s.measures;
        assert_eq!(measures.len(), 3);
        assert_eq!(measures[0].0, 0);
        assert!(approx_eq!(f32, measures[0].1, 0.0));
        assert_eq!(measures[1].0, 3500000000);
        assert!(approx_eq!(
            f32,
            measures[1].1,
            THREADS / (THREADS * 2.0 + 1.0)
        ));
        assert_eq!(measures[2].0, 4500000000);
        assert!(approx_eq!(f32, measures[2].1, 0.0));
        assert_eq!(s.start_ns(), 3500000000);
        assert_eq!(s.end_ns(), 4500000000);
        assert_eq!(s.length_ns(), 1000000000);
        assert!(approx_eq!(f32, s.avg_threads(), 0.4615385));
    }
}
