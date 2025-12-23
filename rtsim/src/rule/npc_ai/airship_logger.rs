use common::rtsim::NpcId;
use hashbrown::HashMap;
use once_cell::sync::Lazy;
use std::{
    env,
    fs::OpenOptions,
    io::Write,
    sync::{Mutex, MutexGuard, PoisonError},
};
use vek::*;
use world::civ::airship_travel::AirshipFlightPhase;

#[derive(Debug)]
/// Facilitates logging airship position changes over time to a log file.
///
/// The generated log file is a CSV file with the following columns:
/// - NpcId
/// - Flight Phase
/// - Time
/// - Position x, y, z
/// - Is Target NPC (boolean, true if the line is for the target NPC)
/// - Is Loaded (boolean, true if the NPC is loaded (not simulated)
///
/// Example log line:
/// NpcId(2171v1), ApproachCruise, 4.538503625, 8529.465, 14783.606, 1475.1228,
/// false, false
///
/// Logging depends on two environment variables:
/// - AIRSHIP_LOGGER_OUTPUT_PATH: The directory where the log file will be
///   created, REQUIRED.
/// - AIRSHIP_LOGGER_TGT_NPC_ID: The NpcId of the target NPC, OPTIONAL. If set,
///   the log data for this NPC will have true in the Is Target NPC column.
pub struct AirshipLogger {
    /// key is NpcId, data is Vec of (flight phase, route index, time, position,
    /// is_tgt_npc, is_loaded)
    pub npc_positions: HashMap<NpcId, Vec<(AirshipFlightPhase, usize, f64, Vec3<f32>, bool, bool)>>,
    /// For tracking when to append the position data to a log file.
    pub start_time: f64,
    /// How often to log the positions, in seconds. Each period, the position
    /// data is cleared so as to avoid excessive memory usage.
    pub interval: f64,
}

impl Default for AirshipLogger {
    fn default() -> Self {
        Self {
            npc_positions: HashMap::default(),
            start_time: 0.0,
            interval: 60.0, // log every 60 seconds
        }
    }
}

/// Synchronization primitive to allowed shared access to the AirshipLogger.
static AIRSHIP_LOGGER: Lazy<Mutex<AirshipLogger>> =
    Lazy::new(|| Mutex::new(AirshipLogger::default()));

/// Accessor for the shared AirshipLogger.
pub fn airship_logger()
-> Result<MutexGuard<'static, AirshipLogger>, PoisonError<MutexGuard<'static, AirshipLogger>>> {
    AIRSHIP_LOGGER.lock()
}

impl AirshipLogger {
    /// Add an airship position to the log. If the time since writing the
    /// last set of positions exceeds self.interval, the positions are
    /// written to a log file in a background thread.
    pub fn log_position(
        &mut self,
        npc_id: NpcId,
        seed: u32,
        route_index: usize,
        phase: &AirshipFlightPhase,
        time: f64,
        position: Vec3<f32>,
        is_loaded: bool,
        value1: f64,
        value2: f64,
    ) {
        let is_tgt_npc = {
            let airship_logger_tgt_npc_id = env::var("AIRSHIP_LOGGER_TGT_NPC_ID").ok();
            if let Some(logger_tgt_npc_id) = airship_logger_tgt_npc_id {
                logger_tgt_npc_id == format!("{:?}", npc_id)
            } else {
                false
            }
        };

        self.npc_positions.entry(npc_id).or_default().push((
            *phase,
            route_index,
            time,
            position,
            is_tgt_npc,
            is_loaded,
        ));
        if self.start_time == 0.0 {
            self.start_time = time;
        } else if time >= self.start_time + self.interval {
            // get the current positions
            let current_positions = self.npc_positions.clone();
            // Start a background thread to add the positions to a file
            std::thread::spawn(move || {
                let airship_logger_output_path = env::var("AIRSHIP_LOGGER_OUTPUT_PATH").ok();
                if let Some(logger_output_path) = airship_logger_output_path {
                    let file_path =
                        format!("{}/airship_positions_{}.log", logger_output_path, seed);
                    let mut file = OpenOptions::new()
                        .append(true) // Enable append mode
                        .create(true) // Create the file if it doesn't exist
                        .open(file_path).expect("Failed to create airship positions log file");
                    for (npc_id, positions) in current_positions {
                        for (phase, route_index, t, pos, is_tgt, is_loaded) in positions {
                            writeln!(
                                file,
                                "{:?}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                                npc_id,
                                route_index,
                                phase,
                                t,
                                pos.x,
                                pos.y,
                                pos.z,
                                is_tgt,
                                is_loaded,
                                value1,
                                value2
                            )
                            .expect("Failed to write to airship positions log file");
                        }
                    }
                }
            });

            // The current positions were cloned and passed to the background thread,
            // clear the current positions and reset the start time.
            self.clear();
            self.start_time = time;
        }
    }

    /// Clear the current logged positions.
    pub fn clear(&mut self) { self.npc_positions.clear(); }
}
