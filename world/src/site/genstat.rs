//! Data structures and functions for tracking site generation statistics.

use crate::util::DHashMap;
use std::{env, fmt, fs::OpenOptions, io::Write};
use tracing::{debug, error};

/// Plot kinds for site generation statistics.
/// These are similar but discrete from the PlotKind enum in the site plot
/// module. For tracking site generation, similar plot kinds are grouped by
/// these enum variants. For example, the House variant includes all kinds of
/// houses (e.g. House, CoastalHouse, DesertCityHouse).
#[derive(Eq, Hash, PartialEq, Copy, Clone)]
pub enum GenStatPlotKind {
    InitialPlaza,
    Plaza,
    Workshop,
    House,
    GuardTower,
    Castle,
    AirshipDock,
    Tavern,
    Yard,
    MultiPlot,
    Temple,
}

impl fmt::Display for GenStatPlotKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            GenStatPlotKind::InitialPlaza => "InitialPlaza",
            GenStatPlotKind::Plaza => "Plaza",
            GenStatPlotKind::Workshop => "Workshop",
            GenStatPlotKind::House => "House",
            GenStatPlotKind::GuardTower => "GuardTower",
            GenStatPlotKind::Castle => "Castle",
            GenStatPlotKind::AirshipDock => "AirshipDock",
            GenStatPlotKind::Tavern => "Tavern",
            GenStatPlotKind::Yard => "Yard",
            GenStatPlotKind::MultiPlot => "MultiPlot",
            GenStatPlotKind::Temple => "Temple",
        };
        write!(f, "{}", s)
    }
}

/// Site kinds for site generation statistics.
/// Only the sites that are tracked for generation statistics are included here,
/// which includes all sites that use the find_roadside_aabr function.
#[derive(Eq, Hash, PartialEq, Copy, Clone, Default)]
pub enum GenStatSiteKind {
    Terracotta,
    Myrmidon,
    #[default]
    City,
    CliffTown,
    SavannahTown,
    CoastalTown,
    DesertCity,
}

impl fmt::Display for GenStatSiteKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            GenStatSiteKind::Terracotta => "Terracotta",
            GenStatSiteKind::Myrmidon => "Myrmidon",
            GenStatSiteKind::City => "City",
            GenStatSiteKind::CliffTown => "CliffTown",
            GenStatSiteKind::SavannahTown => "SavannahTown",
            GenStatSiteKind::CoastalTown => "CoastalTown",
            GenStatSiteKind::DesertCity => "DesertCity",
        };
        write!(f, "{}", s)
    }
}

/// Plot generation statistics.
/// The attempts field increments each time a plot is attempted to be generated.
/// An attempt is counted only once even if find_roadside_aabr is called
/// multiple times.
pub struct GenPlot {
    attempts: u32,
    successful: u32,
}
impl Default for GenPlot {
    fn default() -> Self { Self::new() }
}

impl GenPlot {
    pub fn new() -> Self {
        Self {
            attempts: 0,
            successful: 0,
        }
    }

    pub fn attempt(&mut self) { self.attempts += 1; }

    pub fn success(&mut self) { self.successful += 1; }
}

/// Site generation statistics.
pub struct GenSite {
    kind: GenStatSiteKind,
    name: String,
    stats: DHashMap<GenStatPlotKind, GenPlot>,
}

impl GenSite {
    pub fn new(kind: GenStatSiteKind, name: &str) -> Self {
        Self {
            kind,
            name: name.to_owned(),
            stats: DHashMap::default(),
        }
    }

    pub fn kind(&self) -> &GenStatSiteKind { &self.kind }

    pub fn attempt(&mut self, kind: GenStatPlotKind) {
        self.stats.entry(kind).or_default().attempt();
    }

    pub fn success(&mut self, kind: GenStatPlotKind) {
        self.stats.entry(kind).or_default().success();
    }

    fn at_least(
        &self,
        count: u32,
        plotkind: &GenStatPlotKind,
        genplot: &GenPlot,
        statstr: &mut String,
    ) {
        if genplot.successful < count {
            statstr.push_str(&format!(
                "  {} {} {}: {}/{} GenError: expected at least {}\n",
                self.kind, self.name, plotkind, genplot.successful, genplot.attempts, count
            ));
        }
    }

    fn at_most(
        &self,
        count: u32,
        plotkind: &GenStatPlotKind,
        genplot: &GenPlot,
        statstr: &mut String,
    ) {
        if genplot.successful > count {
            statstr.push_str(&format!(
                "  {} {} {}: {}/{} GenError: expected at most {}\n",
                self.kind, self.name, plotkind, genplot.successful, genplot.attempts, count
            ));
        }
    }

    fn should_not_be_zero(
        &self,
        plotkind: &GenStatPlotKind,
        genplot: &GenPlot,
        statstr: &mut String,
    ) {
        if genplot.successful == 0 {
            statstr.push_str(&format!(
                "  {} {} {}: {}/{} GenWarn: should not be zero\n",
                self.kind, self.name, plotkind, genplot.successful, genplot.attempts
            ));
        }
    }

    fn success_rate(
        &self,
        rate: f32,
        plotkind: &GenStatPlotKind,
        genplot: &GenPlot,
        statstr: &mut String,
    ) {
        if (genplot.successful as f32 / genplot.attempts as f32) < rate {
            statstr.push_str(&format!(
                "  {} {} {}: GenWarn: success rate less than {} ({}/{})\n",
                self.kind, self.name, plotkind, rate, genplot.successful, genplot.attempts
            ));
        }
    }
}

/// World site generation statistics.
/// The map is keyed by site name.
// TODO: This is a bad idea
pub struct SitesGenMeta {
    seed: u32,
    sites: DHashMap<String, GenSite>,
}

fn append_statstr_to_file(file_path: &str, statstr: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;
    file.write_all(statstr.as_bytes())?;
    Ok(())
}

fn get_bool_env_var(var_name: &str) -> bool {
    match env::var(var_name).ok().as_deref() {
        Some("true") => true,
        Some("false") => false,
        _ => false,
    }
}

fn get_log_opts() -> (bool, Option<String>) {
    let site_generation_stats_verbose = get_bool_env_var("SITE_GENERATION_STATS_VERBOSE");
    let site_generation_stats_file_path: Option<String> =
        env::var("SITE_GENERATION_STATS_LOG").ok();
    (
        site_generation_stats_verbose,
        site_generation_stats_file_path,
    )
}

impl SitesGenMeta {
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            sites: DHashMap::default(),
        }
    }

    pub fn add<'a>(&mut self, site_name: impl Into<Option<&'a str>>, kind: GenStatSiteKind) {
        let site_name = site_name.into().unwrap_or("");
        self.sites
            .entry(site_name.to_owned())
            .or_insert_with(|| GenSite::new(kind, site_name));
    }

    pub fn attempt<'a>(&mut self, site_name: impl Into<Option<&'a str>>, kind: GenStatPlotKind) {
        let site_name = site_name.into().unwrap_or("");
        if let Some(gensite) = self.sites.get_mut(site_name) {
            gensite.attempt(kind);
        } else {
            error!("Site not found: {}", site_name);
        }
    }

    pub fn success<'a>(&mut self, site_name: impl Into<Option<&'a str>>, kind: GenStatPlotKind) {
        let site_name = site_name.into().unwrap_or("");
        if let Some(gensite) = self.sites.get_mut(site_name) {
            gensite.success(kind);
        } else {
            error!("Site not found: {}", site_name);
        }
    }

    /// Log the site generation statistics.
    /// Nothing is logged unless the RUST_LOG environment variable is set to
    /// DEBUG. Two additional environment variables can be set to control
    /// the output: SITE_GENERATION_STATS_VERBOSE: If set to true, the
    /// output will include everything shown in the output format below.
    /// If set to false or not set, only generation errors will be shown.
    /// SITE_GENERATION_STATS_LOG: If set, the output will be appended to the
    /// file at the path specified by this variable. The value must be a
    /// valid absolute or relative path (from the current working directory),
    /// including the file name. The file will be created if it does not
    /// exist.
    pub fn log(&self) {
        // Get the current tracing log level
        // This can be set with the RUST_LOG environment variable.
        let current_log_level = tracing::level_filters::LevelFilter::current();
        if current_log_level == tracing::Level::DEBUG {
            let (verbose, log_path) = get_log_opts();

            /*
               For each world generated, gather this information:
                   seed
                   Number of sites generated
                   Number of each site kind generated
                   For Each Site
                       Number of plots generated (success/attempts)
                       Number of each plot kind generated

               Output format
                   ------------------ SitesGenMeta seed  12345
                   Number of sites: 7
                       Terracotta: 5
                       Myrmidon: 2
                       City: 8
                   Terracotta <Town Name>
                       Number of plots: 4
                           InitialPlaza: 1/1
                           Plaza: 1/3
                           House: 1/1
                           ...
                       GenErrors
                       GenWarnings
                   City <Town Name>
                       Number of plots: 4
                           InitialPlaza: 1/1
                           Plaza: 1/3
                           House: 1/1
                           ...
                       GenErrors
                       GenWarnings
            */
            let mut num_sites: u32 = 0;
            let mut site_counts: DHashMap<GenStatSiteKind, u32> = DHashMap::default();
            let mut stat_stat_str = String::new();
            for (_, gensite) in self.sites.iter() {
                num_sites += 1;
                *site_counts.entry(*gensite.kind()).or_insert(0) += 1;
            }
            stat_stat_str.push_str(&format!(
                "------------------ SitesGenMeta seed {}\n",
                self.seed
            ));
            if verbose {
                stat_stat_str.push_str(&format!("Sites: {}\n", num_sites));
                for (site_kind, count) in site_counts.iter() {
                    stat_stat_str.push_str(&format!("  {}: {}\n", site_kind, count));
                }
            }
            for (site_name, gensite) in self.sites.iter() {
                let mut stat_err_str = String::new();
                let mut stat_warn_str = String::new();
                let mut num_plots: u32 = 0;
                let mut plot_counts: DHashMap<GenStatPlotKind, (u32, u32)> = DHashMap::default();
                for (plotkind, genplot) in gensite.stats.iter() {
                    num_plots += 1;
                    plot_counts.entry(*plotkind).or_insert((0, 0)).0 += genplot.successful;
                    plot_counts.entry(*plotkind).or_insert((0, 0)).1 += genplot.attempts;
                }
                match &gensite.kind() {
                    GenStatSiteKind::Terracotta => {
                        for (kind, genplot) in gensite.stats.iter() {
                            match &kind {
                                GenStatPlotKind::InitialPlaza => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                },
                                GenStatPlotKind::Plaza => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                },
                                GenStatPlotKind::House => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                    gensite.success_rate(0.1, kind, genplot, &mut stat_warn_str);
                                },
                                GenStatPlotKind::Yard => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                },
                                _ => {},
                            }
                        }
                    },
                    GenStatSiteKind::Myrmidon => {
                        for (kind, genplot) in gensite.stats.iter() {
                            match &kind {
                                GenStatPlotKind::InitialPlaza => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                },
                                GenStatPlotKind::Plaza => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                },
                                GenStatPlotKind::House => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                    gensite.success_rate(0.1, kind, genplot, &mut stat_warn_str);
                                },
                                _ => {},
                            }
                        }
                    },
                    GenStatSiteKind::City => {
                        for (kind, genplot) in gensite.stats.iter() {
                            match &kind {
                                GenStatPlotKind::InitialPlaza => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                },
                                GenStatPlotKind::Plaza => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                },
                                GenStatPlotKind::Workshop => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                },
                                GenStatPlotKind::House => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                    gensite.success_rate(0.2, kind, genplot, &mut stat_warn_str);
                                },
                                _ => {},
                            }
                        }
                    },
                    GenStatSiteKind::CliffTown => {
                        for (kind, genplot) in gensite.stats.iter() {
                            match &kind {
                                GenStatPlotKind::InitialPlaza => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                },
                                GenStatPlotKind::Plaza => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                },
                                GenStatPlotKind::House => {
                                    gensite.at_least(5, kind, genplot, &mut stat_err_str);
                                    gensite.success_rate(0.5, kind, genplot, &mut stat_warn_str);
                                },
                                GenStatPlotKind::AirshipDock => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                    gensite.success_rate(0.1, kind, genplot, &mut stat_warn_str);
                                },
                                _ => {},
                            }
                        }
                    },
                    GenStatSiteKind::SavannahTown => {
                        for (kind, genplot) in gensite.stats.iter() {
                            match &kind {
                                GenStatPlotKind::InitialPlaza => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                },
                                GenStatPlotKind::Plaza => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                },
                                GenStatPlotKind::Workshop => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                },
                                GenStatPlotKind::House => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                    gensite.success_rate(0.5, kind, genplot, &mut stat_warn_str);
                                },
                                GenStatPlotKind::AirshipDock => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                },
                                _ => {},
                            }
                        }
                    },
                    GenStatSiteKind::CoastalTown => {
                        for (kind, genplot) in gensite.stats.iter() {
                            match &kind {
                                GenStatPlotKind::InitialPlaza => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                },
                                GenStatPlotKind::Plaza => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                },
                                GenStatPlotKind::Workshop => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                },
                                GenStatPlotKind::House => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                    gensite.success_rate(0.5, kind, genplot, &mut stat_warn_str);
                                },
                                GenStatPlotKind::AirshipDock => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                    gensite.at_most(1, kind, genplot, &mut stat_err_str);
                                },
                                _ => {},
                            }
                        }
                    },
                    GenStatSiteKind::DesertCity => {
                        for (kind, genplot) in gensite.stats.iter() {
                            match &kind {
                                GenStatPlotKind::InitialPlaza => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                },
                                GenStatPlotKind::Plaza => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                },
                                GenStatPlotKind::MultiPlot => {
                                    gensite.at_least(1, kind, genplot, &mut stat_err_str);
                                },
                                GenStatPlotKind::Temple => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                },
                                GenStatPlotKind::AirshipDock => {
                                    gensite.should_not_be_zero(kind, genplot, &mut stat_warn_str);
                                },
                                _ => {},
                            }
                        }
                    },
                }
                if verbose {
                    stat_stat_str.push_str(&format!("{} {}\n", gensite.kind(), site_name));
                    stat_stat_str.push_str(&format!("  Number of plots: {}\n", num_plots));
                    for (plotkind, count) in plot_counts.iter() {
                        stat_stat_str
                            .push_str(&format!("  {}: {}/{}\n", plotkind, count.0, count.1));
                    }
                }
                if !stat_err_str.is_empty() {
                    stat_stat_str.push_str(&stat_err_str.to_string());
                }
                if verbose && !stat_warn_str.is_empty() {
                    stat_stat_str.push_str(&stat_warn_str.to_string());
                }
            }
            debug!("{}", stat_stat_str);
            if let Some(log_path) = log_path {
                if let Err(e) = append_statstr_to_file(&log_path, &stat_stat_str) {
                    eprintln!("Failed to write to file: {}", e);
                } else {
                    println!("Statistics written to {}", log_path);
                }
            }
        }
    }
}
