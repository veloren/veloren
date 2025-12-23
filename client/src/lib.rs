#![deny(unsafe_code)]
#![deny(clippy::clone_on_ref_ptr)]

pub mod addr;
pub mod error;

// Reexports
pub use crate::error::Error;
pub use authc::AuthClientError;
pub use common_net::msg::ServerInfo;
pub use specs::{
    Builder, DispatcherBuilder, Entity as EcsEntity, Join, LendJoin, ReadStorage, World, WorldExt,
};

use crate::addr::ConnectionArgs;
use byteorder::{ByteOrder, LittleEndian};
use common::{
    character::{CharacterId, CharacterItem},
    comp::{
        self, AdminRole, CharacterState, ChatMode, ControlAction, ControlEvent, Controller,
        ControllerInputs, GroupManip, Hardcore, InputKind, InventoryAction, InventoryEvent,
        InventoryUpdateEvent, MapMarkerChange, PresenceKind, UtteranceKind,
        chat::KillSource,
        controller::CraftEvent,
        gizmos::Gizmos,
        group,
        inventory::{
            InventorySortOrder,
            item::{ItemKind, modular, tool},
        },
        invite::{InviteKind, InviteResponse},
        skills::Skill,
        slot::{EquipSlot, InvSlotId, Slot},
    },
    event::{EventBus, LocalEvent, PluginHash, UpdateCharacterMetadata},
    grid::Grid,
    link::Is,
    lod,
    map::Marker,
    mounting::{Rider, VolumePos, VolumeRider},
    outcome::Outcome,
    recipe::{ComponentRecipeBook, RecipeBookManifest, RepairRecipeBook},
    resources::{BattleMode, GameMode, PlayerEntity, Time, TimeOfDay},
    rtsim,
    shared_server_config::ServerConstants,
    spiral::Spiral2d,
    terrain::{
        BiomeKind, CoordinateConversions, SiteKindMeta, SpriteKind, TerrainChunk, TerrainChunkSize,
        TerrainGrid, block::Block, map::MapConfig, neighbors,
    },
    trade::{PendingTrade, SitePrices, TradeAction, TradeId, TradeResult},
    uid::{IdMaps, Uid},
    vol::RectVolSize,
    weather::{CompressedWeather, SharedWeatherGrid, Weather, WeatherGrid},
};
#[cfg(feature = "tracy")] use common_base::plot;
use common_base::{prof_span, span};
use common_i18n::Content;
use common_net::{
    msg::{
        ChatTypeContext, ClientGeneral, ClientMsg, ClientRegister, DisconnectReason, InviteAnswer,
        Notification, PingMsg, PlayerInfo, PlayerListUpdate, RegisterError, ServerGeneral,
        ServerInit, ServerRegisterAnswer,
        server::ServerDescription,
        world_msg::{EconomyInfo, PoiInfo, SiteId},
    },
    sync::WorldSyncExt,
};

pub use common_net::msg::ClientType;
use common_state::State;
#[cfg(feature = "plugins")]
use common_state::plugin::PluginMgr;
use common_systems::add_local_systems;
use comp::BuffKind;
use hashbrown::{HashMap, HashSet};
use hickory_resolver::{Resolver, config::ResolverConfig, name_server::TokioConnectionProvider};
use image::DynamicImage;
use network::{ConnectAddr, Network, Participant, Pid, Stream};
use num::traits::FloatConst;
use rayon::prelude::*;
use rustls::client::danger::ServerCertVerified;
use specs::Component;
use std::{
    collections::{BTreeMap, VecDeque},
    fmt::Debug,
    mem,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;
use tracing::{debug, error, trace, warn};
use vek::*;

pub const MAX_SELECTABLE_VIEW_DISTANCE: u32 = 65;

const PING_ROLLING_AVERAGE_SECS: usize = 10;

/// Client frontend events.
///
/// These events are returned to the frontend that ticks the client.
#[derive(Debug)]
pub enum Event {
    Chat(comp::ChatMsg),
    GroupInventoryUpdate(comp::FrontendItem, Uid),
    InviteComplete {
        target: Uid,
        answer: InviteAnswer,
        kind: InviteKind,
    },
    TradeComplete {
        result: TradeResult,
        trade: PendingTrade,
    },
    Disconnect,
    DisconnectionNotification(u64),
    InventoryUpdated(Vec<InventoryUpdateEvent>),
    Notification(UserNotification),
    SetViewDistance(u32),
    Outcome(Outcome),
    CharacterCreated(CharacterId),
    CharacterEdited(CharacterId),
    CharacterJoined(UpdateCharacterMetadata),
    CharacterError(String),
    MapMarker(comp::MapMarkerUpdate),
    StartSpectate(Vec3<f32>),
    SpectatePosition(Vec3<f32>),
    PluginDataReceived(Vec<u8>),
    Dialogue(Uid, rtsim::Dialogue<true>),
    Gizmos(Vec<Gizmos>),
}

/// A message for the user to be displayed through the UI.
///
/// This type mirrors the [`common_net::msg::Notification`] type, but does not
/// include any data that the UI does not need.
#[derive(Debug)]
pub enum UserNotification {
    WaypointUpdated,
}

#[derive(Debug)]
pub enum ClientInitStage {
    /// A connection to the server is being created
    ConnectionEstablish,
    /// Waiting for server version
    WatingForServerVersion,
    /// We're currently authenticating with the server
    Authentication,
    /// Loading map data, site information, recipe information and other
    /// initialization data
    LoadingInitData,
    /// Prepare data received by the server to be used by the client (insert
    /// data into the ECS, render map)
    StartingClient,
}

pub struct WorldData {
    /// Just the "base" layer for LOD; currently includes colors and nothing
    /// else. In the future we'll add more layers, like shadows, rivers, and
    /// probably foliage, cities, roads, and other structures.
    pub lod_base: Grid<u32>,
    /// The "height" layer for LOD; currently includes only land altitudes, but
    /// in the future should also water depth, and probably other
    /// information as well.
    pub lod_alt: Grid<u32>,
    /// The "shadow" layer for LOD.  Includes east and west horizon angles and
    /// an approximate max occluder height, which we use to try to
    /// approximate soft and volumetric shadows.
    pub lod_horizon: Grid<u32>,
    /// A fully rendered map image for use with the map and minimap; note that
    /// this can be constructed dynamically by combining the layers of world
    /// map data (e.g. with shadow map data or river data), but at present
    /// we opt not to do this.
    ///
    /// The first two elements of the tuple are the regular and topographic maps
    /// respectively. The third element of the tuple is the world size (as a 2D
    /// grid, in chunks), and the fourth element holds the minimum height for
    /// any land chunk (i.e. the sea level) in its x coordinate, and the maximum
    /// land height above this height (i.e. the max height) in its y coordinate.
    map: (Vec<Arc<DynamicImage>>, Vec2<u16>, Vec2<f32>),
}

impl WorldData {
    pub fn chunk_size(&self) -> Vec2<u16> { self.map.1 }

    pub fn map_layers(&self) -> &Vec<Arc<DynamicImage>> { &self.map.0 }

    pub fn map_image(&self) -> &Arc<DynamicImage> { &self.map.0[0] }

    pub fn topo_map_image(&self) -> &Arc<DynamicImage> { &self.map.0[1] }

    pub fn min_chunk_alt(&self) -> f32 { self.map.2.x }

    pub fn max_chunk_alt(&self) -> f32 { self.map.2.y }
}

pub struct SiteMarker {
    pub marker: Marker,
    pub economy: Option<EconomyInfo>,
}

struct WeatherLerp {
    old: (SharedWeatherGrid, Instant),
    new: (SharedWeatherGrid, Instant),
    old_local_wind: (Vec2<f32>, Instant),
    new_local_wind: (Vec2<f32>, Instant),
    local_wind: Vec2<f32>,
}

impl WeatherLerp {
    fn local_wind_update(&mut self, wind: Vec2<f32>) {
        self.old_local_wind = mem::replace(&mut self.new_local_wind, (wind, Instant::now()));
    }

    fn update_local_wind(&mut self) {
        // Assumes updates are regular
        let t = (self.new_local_wind.1.elapsed().as_secs_f32()
            / self
                .new_local_wind
                .1
                .duration_since(self.old_local_wind.1)
                .as_secs_f32())
        .clamp(0.0, 1.0);

        self.local_wind = Vec2::lerp_unclamped(self.old_local_wind.0, self.new_local_wind.0, t);
    }

    fn weather_update(&mut self, weather: SharedWeatherGrid) {
        self.old = mem::replace(&mut self.new, (weather, Instant::now()));
    }

    // TODO: Make improvements to this interpolation, it's main issue is assuming
    // that updates come at regular intervals.
    fn update(&mut self, to_update: &mut WeatherGrid) {
        prof_span!("WeatherLerp::update");
        self.update_local_wind();
        let old = &self.old.0;
        let new = &self.new.0;
        if new.size() == Vec2::zero() {
            return;
        }
        if to_update.size() != new.size() {
            *to_update = WeatherGrid::from(new);
        }
        if old.size() == new.size() {
            // Assumes updates are regular
            let t = (self.new.1.elapsed().as_secs_f32()
                / self.new.1.duration_since(self.old.1).as_secs_f32())
            .clamp(0.0, 1.0);

            to_update
                .iter_mut()
                .zip(old.iter().zip(new.iter()))
                .for_each(|((_, current), ((_, old), (_, new)))| {
                    *current = CompressedWeather::lerp_unclamped(old, new, t);
                    // `local_wind` is set for all weather cells on the client,
                    // which will still be inaccurate outside the "local" area
                    current.wind = self.local_wind;
                });
        }
    }
}

impl Default for WeatherLerp {
    fn default() -> Self {
        let old = Instant::now();
        let new = Instant::now();
        Self {
            old: (SharedWeatherGrid::new(Vec2::zero()), old),
            new: (SharedWeatherGrid::new(Vec2::zero()), new),
            old_local_wind: (Vec2::zero(), old),
            new_local_wind: (Vec2::zero(), new),
            local_wind: Vec2::zero(),
        }
    }
}

pub struct Client {
    client_type: ClientType,
    registered: bool,
    presence: Option<PresenceKind>,
    runtime: Arc<Runtime>,
    server_info: ServerInfo,
    /// Localized server motd and rules
    server_description: ServerDescription,
    world_data: WorldData,
    weather: WeatherLerp,
    player_list: HashMap<Uid, PlayerInfo>,
    character_list: CharacterList,
    character_being_deleted: Option<CharacterId>,
    sites: HashMap<SiteId, SiteMarker>,
    extra_markers: Vec<Marker>,
    possible_starting_sites: Vec<SiteId>,
    pois: Vec<PoiInfo>,
    pub chat_mode: ChatMode,
    component_recipe_book: ComponentRecipeBook,
    repair_recipe_book: RepairRecipeBook,
    available_recipes: HashMap<String, Option<SpriteKind>>,
    lod_zones: HashMap<Vec2<i32>, lod::Zone>,
    lod_last_requested: Option<Instant>,
    lod_pos_fallback: Option<Vec2<f32>>,
    force_update_counter: u64,

    role: Option<AdminRole>,
    max_group_size: u32,
    // Client has received an invite (inviter uid, time out instant)
    invite: Option<(Uid, Instant, Duration, InviteKind)>,
    group_leader: Option<Uid>,
    // Note: potentially representable as a client only component
    group_members: HashMap<Uid, group::Role>,
    // Pending invites that this client has sent out
    pending_invites: HashSet<Uid>,
    // The pending trade the client is involved in, and it's id
    pending_trade: Option<(TradeId, PendingTrade, Option<SitePrices>)>,
    waypoint: Option<String>,

    network: Option<Network>,
    participant: Option<Participant>,
    general_stream: Stream,
    ping_stream: Stream,
    register_stream: Stream,
    character_screen_stream: Stream,
    in_game_stream: Stream,
    terrain_stream: Stream,

    client_timeout: Duration,
    last_server_ping: f64,
    last_server_pong: f64,
    last_ping_delta: f64,
    ping_deltas: VecDeque<f64>,

    tick: u64,
    state: State,

    flashing_lights_enabled: bool,

    /// Terrrain view distance
    server_view_distance_limit: Option<u32>,
    view_distance: Option<u32>,
    lod_distance: f32,
    // TODO: move into voxygen
    loaded_distance: f32,

    pending_chunks: HashMap<Vec2<i32>, Instant>,
    target_time_of_day: Option<TimeOfDay>,
    dt_adjustment: f64,

    connected_server_constants: ServerConstants,
    /// Requested but not yet received plugins
    missing_plugins: HashSet<PluginHash>,
    /// Locally cached plugins needed by the server
    local_plugins: Vec<PathBuf>,
}

/// Holds data related to the current players characters, as well as some
/// additional state to handle UI.
#[derive(Debug, Default)]
pub struct CharacterList {
    pub characters: Vec<CharacterItem>,
    pub loading: bool,
}

async fn connect_quic(
    network: &Network,
    hostname: String,
    override_port: Option<u16>,
    prefer_ipv6: bool,
    validate_tls: bool,
) -> Result<network::Participant, crate::error::Error> {
    let config = if validate_tls {
        quinn::ClientConfig::try_with_platform_verifier()?
    } else {
        warn!(
            "skipping validation of server identity. There is no guarantee that the server you're \
             connected to is the one you expect to be connecting to."
        );
        #[derive(Debug)]
        struct Verifier;
        impl rustls::client::danger::ServerCertVerifier for Verifier {
            fn verify_server_cert(
                &self,
                _end_entity: &rustls::pki_types::CertificateDer<'_>,
                _intermediates: &[rustls::pki_types::CertificateDer<'_>],
                _server_name: &rustls::pki_types::ServerName<'_>,
                _ocsp_response: &[u8],
                _now: rustls::pki_types::UnixTime,
            ) -> Result<ServerCertVerified, rustls::Error> {
                Ok(ServerCertVerified::assertion())
            }

            fn verify_tls12_signature(
                &self,
                _message: &[u8],
                _cert: &rustls::pki_types::CertificateDer<'_>,
                _dss: &rustls::DigitallySignedStruct,
            ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error>
            {
                Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
            }

            fn verify_tls13_signature(
                &self,
                _message: &[u8],
                _cert: &rustls::pki_types::CertificateDer<'_>,
                _dss: &rustls::DigitallySignedStruct,
            ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error>
            {
                Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
            }

            fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
                vec![
                    rustls::SignatureScheme::RSA_PKCS1_SHA1,
                    rustls::SignatureScheme::ECDSA_SHA1_Legacy,
                    rustls::SignatureScheme::RSA_PKCS1_SHA256,
                    rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
                    rustls::SignatureScheme::RSA_PKCS1_SHA384,
                    rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
                    rustls::SignatureScheme::RSA_PKCS1_SHA512,
                    rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
                    rustls::SignatureScheme::RSA_PSS_SHA256,
                    rustls::SignatureScheme::RSA_PSS_SHA384,
                    rustls::SignatureScheme::RSA_PSS_SHA512,
                    rustls::SignatureScheme::ED25519,
                    rustls::SignatureScheme::ED448,
                ]
            }
        }

        let mut cfg = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(Verifier))
            .with_no_client_auth();
        cfg.enable_early_data = true;

        quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(cfg).unwrap(),
        ))
    };

    addr::try_connect(network, &hostname, override_port, prefer_ipv6, |a| {
        ConnectAddr::Quic(a, config.clone(), hostname.clone())
    })
    .await
}

impl Client {
    pub async fn new(
        addr: ConnectionArgs,
        runtime: Arc<Runtime>,
        // TODO: refactor to avoid needing to use this out parameter
        mismatched_server_info: &mut Option<ServerInfo>,
        username: &str,
        password: &str,
        locale: Option<String>,
        auth_trusted: impl FnMut(&str) -> bool,
        init_stage_update: &(dyn Fn(ClientInitStage) + Send + Sync),
        add_foreign_systems: impl Fn(&mut DispatcherBuilder) + Send + 'static,
        #[cfg_attr(not(feature = "plugins"), expect(unused_variables))] config_dir: PathBuf,
        client_type: ClientType,
    ) -> Result<Self, Error> {
        let _ = rustls::crypto::ring::default_provider().install_default(); // needs to be initialized before usage
        let network = Network::new(Pid::new(), &runtime);

        init_stage_update(ClientInitStage::ConnectionEstablish);

        let mut participant = match addr {
            ConnectionArgs::Srv {
                hostname,
                prefer_ipv6,
                validate_tls,
                use_quic,
            } => {
                // Try to create a resolver backed by /etc/resolv.conf or the Windows Registry
                // first. If that fails, create a resolver being hard-coded to
                // Google's 8.8.8.8 public resolver.
                let resolver = Resolver::builder_tokio()
                    .unwrap_or_else(|error| {
                        error!(
                            "Failed to create DNS resolver using system configuration: {error:?}"
                        );
                        warn!("Falling back to a default configured resolver.");
                        Resolver::builder_with_config(
                            ResolverConfig::default(),
                            TokioConnectionProvider::default(),
                        )
                    })
                    .build();

                let quic_service_host = format!("_veloren._udp.{hostname}");
                let quic_lookup_future = resolver.srv_lookup(quic_service_host);
                let tcp_service_host = format!("_veloren._tcp.{hostname}");
                let tcp_lookup_future = resolver.srv_lookup(tcp_service_host);
                let (quic_rr, tcp_rr) = tokio::join!(quic_lookup_future, tcp_lookup_future);

                #[derive(Eq, PartialEq)]
                enum ConnMode {
                    Quic,
                    Tcp,
                }

                // Push the results of both futures into `srv_rr`. This uses map_or_else purely
                // for side effects.
                let mut srv_rr = Vec::new();
                let () = quic_rr.map_or_else(
                    |error| {
                        warn!("QUIC SRV lookup failed: {error:?}");
                    },
                    |srv_lookup| {
                        srv_rr.extend(srv_lookup.iter().cloned().map(|srv| (ConnMode::Quic, srv)))
                    },
                );
                let () = tcp_rr.map_or_else(
                    |error| {
                        warn!("TCP SRV lookup failed: {error:?}");
                    },
                    |srv_lookup| {
                        srv_rr.extend(srv_lookup.iter().cloned().map(|srv| (ConnMode::Tcp, srv)))
                    },
                );

                // SRV records have a priority; lowest priority hosts MUST be contacted first.
                let srv_rr_slice = srv_rr.as_mut_slice();
                srv_rr_slice.sort_by_key(|(_, srv)| srv.priority());

                let mut iter = srv_rr_slice.iter();

                // This loops exits as soon as the above iter over `srv_rr_slice` is exhausted
                loop {
                    if let Some((conn_mode, srv_rr)) = iter.next() {
                        let hostname = format!("{}", srv_rr.target());
                        let port = Some(srv_rr.port());
                        let conn_result = match conn_mode {
                            ConnMode::Quic => {
                                connect_quic(&network, hostname, port, prefer_ipv6, validate_tls)
                                    .await
                            },
                            ConnMode::Tcp => {
                                addr::try_connect(
                                    &network,
                                    &hostname,
                                    port,
                                    prefer_ipv6,
                                    ConnectAddr::Tcp,
                                )
                                .await
                            },
                        };
                        match conn_result {
                            Ok(c) => break c,
                            Err(error) => {
                                warn!("Failed to connect to host {}: {error:?}", srv_rr.target())
                            },
                        }
                    } else {
                        warn!(
                            "No SRV hosts succeeded connection, falling back to direct connection"
                        );
                        // This case is also hit if no SRV host was returned from the query, so we
                        // check for QUIC/TCP preference.
                        let c = if use_quic {
                            connect_quic(&network, hostname, None, prefer_ipv6, validate_tls)
                                .await?
                        } else {
                            match addr::try_connect(
                                &network,
                                &hostname,
                                None,
                                prefer_ipv6,
                                ConnectAddr::Tcp,
                            )
                            .await
                            {
                                Ok(c) => c,
                                Err(error) => return Err(error),
                            }
                        };
                        break c;
                    }
                }
            },
            ConnectionArgs::Tcp {
                hostname,
                prefer_ipv6,
            } => {
                addr::try_connect(&network, &hostname, None, prefer_ipv6, ConnectAddr::Tcp).await?
            },
            ConnectionArgs::Quic {
                hostname,
                prefer_ipv6,
                validate_tls,
            } => {
                warn!(
                    "QUIC is enabled. This is experimental and you won't be able to connect to \
                     TCP servers unless deactivated"
                );

                connect_quic(&network, hostname, None, prefer_ipv6, validate_tls).await?
            },
            ConnectionArgs::Mpsc(id) => network.connect(ConnectAddr::Mpsc(id)).await?,
        };

        let stream = participant.opened().await?;
        let ping_stream = participant.opened().await?;
        let mut register_stream = participant.opened().await?;
        let character_screen_stream = participant.opened().await?;
        let in_game_stream = participant.opened().await?;
        let terrain_stream = participant.opened().await?;

        init_stage_update(ClientInitStage::WatingForServerVersion);
        register_stream.send(client_type)?;
        let server_info: ServerInfo = register_stream.recv().await?;
        if server_info.git_hash != *common::util::GIT_HASH
            || server_info.git_timestamp != *common::util::GIT_TIMESTAMP
        {
            warn!(
                "Server is running {}, you are running {}, versions might be incompatible!",
                common::util::make_display_version(server_info.git_hash, server_info.git_timestamp),
                *common::util::DISPLAY_VERSION,
            );
        }
        // Pass the server info back to the caller to ensure they can access it even
        // if this function errors.
        *mismatched_server_info = Some(server_info.clone());
        debug!("Auth Server: {:?}", server_info.auth_provider);

        ping_stream.send(PingMsg::Ping)?;

        init_stage_update(ClientInitStage::Authentication);
        // Register client
        Self::register(
            username,
            password,
            locale,
            auth_trusted,
            &server_info,
            &mut register_stream,
        )
        .await?;

        init_stage_update(ClientInitStage::LoadingInitData);
        // Wait for initial sync
        let mut ping_interval = tokio::time::interval(Duration::from_secs(1));
        let ServerInit::GameSync {
            entity_package,
            time_of_day,
            max_group_size,
            client_timeout,
            world_map,
            recipe_book,
            component_recipe_book,
            material_stats,
            ability_map,
            server_constants,
            repair_recipe_book,
            description,
            active_plugins: _active_plugins,
            role,
        } = loop {
            tokio::select! {
                // Spawn in a blocking thread (leaving the network thread free).  This is mostly
                // useful for bots.
                res = register_stream.recv() => break res?,
                _ = ping_interval.tick() => ping_stream.send(PingMsg::Ping)?,
            }
        };

        init_stage_update(ClientInitStage::StartingClient);
        // Spawn in a blocking thread (leaving the network thread free).  This is mostly
        // useful for bots.
        let mut task = tokio::task::spawn_blocking(move || {
            let map_size_lg =
                common::terrain::MapSizeLg::new(world_map.dimensions_lg).map_err(|_| {
                    Error::Other(format!(
                        "Server sent bad world map dimensions: {:?}",
                        world_map.dimensions_lg,
                    ))
                })?;
            let sea_level = world_map.default_chunk.get_min_z() as f32;

            // Initialize `State`
            let pools = State::pools(GameMode::Client);
            let mut state = State::client(
                pools,
                map_size_lg,
                world_map.default_chunk,
                // TODO: Add frontend systems
                |dispatch_builder| {
                    add_local_systems(dispatch_builder);
                    add_foreign_systems(dispatch_builder);
                },
                #[cfg(feature = "plugins")]
                common_state::plugin::PluginMgr::from_asset_or_default(),
            );

            #[cfg_attr(not(feature = "plugins"), expect(unused_mut))]
            let mut missing_plugins: Vec<PluginHash> = Vec::new();
            #[cfg_attr(not(feature = "plugins"), expect(unused_mut))]
            let mut local_plugins: Vec<PathBuf> = Vec::new();
            #[cfg(feature = "plugins")]
            {
                let already_present = state.ecs().read_resource::<PluginMgr>().plugin_list();
                for hash in _active_plugins.iter() {
                    if !already_present.contains(hash) {
                        // look in config_dir first (cache)
                        if let Ok(local_path) = common_state::plugin::find_cached(&config_dir, hash)
                        {
                            local_plugins.push(local_path);
                        } else {
                            //tracing::info!("cache not found {local_path:?}");
                            tracing::info!("Server requires plugin {hash:x?}");
                            missing_plugins.push(*hash);
                        }
                    }
                }
            }
            // Client-only components
            state.ecs_mut().register::<comp::Last<CharacterState>>();
            let entity = state.ecs_mut().apply_entity_package(entity_package);
            *state.ecs_mut().write_resource() = time_of_day;
            *state.ecs_mut().write_resource() = PlayerEntity(Some(entity));
            state.ecs_mut().insert(material_stats);
            state.ecs_mut().insert(ability_map);
            state.ecs_mut().insert(recipe_book);

            let map_size = map_size_lg.chunks();
            let max_height = world_map.max_height;
            let rgba = world_map.rgba;
            let alt = world_map.alt;
            if rgba.size() != map_size.map(|e| e as i32) {
                return Err(Error::Other("Server sent a bad world map image".into()));
            }
            if alt.size() != map_size.map(|e| e as i32) {
                return Err(Error::Other("Server sent a bad altitude map.".into()));
            }
            let [west, east] = world_map.horizons;
            let scale_angle = |a: u8| (a as f32 / 255.0 * <f32 as FloatConst>::FRAC_PI_2()).tan();
            let scale_height = |h: u8| h as f32 / 255.0 * max_height;
            let scale_height_big = |h: u32| (h >> 3) as f32 / 8191.0 * max_height;

            debug!("Preparing image...");
            let unzip_horizons = |(angles, heights): &(Vec<_>, Vec<_>)| {
                (
                    angles.iter().copied().map(scale_angle).collect::<Vec<_>>(),
                    heights
                        .iter()
                        .copied()
                        .map(scale_height)
                        .collect::<Vec<_>>(),
                )
            };
            let horizons = [unzip_horizons(&west), unzip_horizons(&east)];

            // Redraw map (with shadows this time).
            let mut world_map_rgba = vec![0u32; rgba.size().product() as usize];
            let mut world_map_topo = vec![0u32; rgba.size().product() as usize];
            let mut map_config = common::terrain::map::MapConfig::orthographic(
                map_size_lg,
                core::ops::RangeInclusive::new(0.0, max_height),
            );
            map_config.horizons = Some(&horizons);
            let rescale_height = |h: f32| h / max_height;
            let bounds_check = |pos: Vec2<i32>| {
                pos.reduce_partial_min() >= 0
                    && pos.x < map_size.x as i32
                    && pos.y < map_size.y as i32
            };
            fn sample_pos(
                map_config: &MapConfig,
                pos: Vec2<i32>,
                alt: &Grid<u32>,
                rgba: &Grid<u32>,
                map_size: &Vec2<u16>,
                map_size_lg: &common::terrain::MapSizeLg,
                max_height: f32,
            ) -> common::terrain::map::MapSample {
                let rescale_height = |h: f32| h / max_height;
                let scale_height_big = |h: u32| (h >> 3) as f32 / 8191.0 * max_height;
                let bounds_check = |pos: Vec2<i32>| {
                    pos.reduce_partial_min() >= 0
                        && pos.x < map_size.x as i32
                        && pos.y < map_size.y as i32
                };
                let MapConfig {
                    gain,
                    is_contours,
                    is_height_map,
                    is_stylized_topo,
                    ..
                } = *map_config;
                let mut is_contour_line = false;
                let mut is_border = false;
                let (rgb, alt, downhill_wpos) = if bounds_check(pos) {
                    let posi = pos.y as usize * map_size.x as usize + pos.x as usize;
                    let [r, g, b, _a] = rgba[pos].to_le_bytes();
                    let is_water = r == 0 && b > 102 && g < 77;
                    let alti = alt[pos];
                    // Compute contours (chunks are assigned in the river code below)
                    let altj = rescale_height(scale_height_big(alti));
                    let contour_interval = 150.0;
                    let chunk_contour = (altj * gain / contour_interval) as u32;

                    // Compute downhill.
                    let downhill = {
                        let mut best = -1;
                        let mut besth = alti;
                        for nposi in neighbors(*map_size_lg, posi) {
                            let nbh = alt.raw()[nposi];
                            let nalt = rescale_height(scale_height_big(nbh));
                            let nchunk_contour = (nalt * gain / contour_interval) as u32;
                            if !is_contour_line && chunk_contour > nchunk_contour {
                                is_contour_line = true;
                            }
                            let [nr, ng, nb, _na] = rgba.raw()[nposi].to_le_bytes();
                            let n_is_water = nr == 0 && nb > 102 && ng < 77;

                            if !is_border && is_water && !n_is_water {
                                is_border = true;
                            }

                            if nbh < besth {
                                besth = nbh;
                                best = nposi as isize;
                            }
                        }
                        best
                    };
                    let downhill_wpos = if downhill < 0 {
                        None
                    } else {
                        Some(
                            Vec2::new(
                                (downhill as usize % map_size.x as usize) as i32,
                                (downhill as usize / map_size.x as usize) as i32,
                            ) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32),
                        )
                    };
                    (Rgb::new(r, g, b), alti, downhill_wpos)
                } else {
                    (Rgb::zero(), 0, None)
                };
                let alt = f64::from(rescale_height(scale_height_big(alt)));
                let wpos = pos * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);
                let downhill_wpos =
                    downhill_wpos.unwrap_or(wpos + TerrainChunkSize::RECT_SIZE.map(|e| e as i32));
                let is_path = rgb.r == 0x37 && rgb.g == 0x29 && rgb.b == 0x23;
                let rgb = rgb.map(|e: u8| e as f64 / 255.0);
                let is_water = rgb.r == 0.0 && rgb.b > 0.4 && rgb.g < 0.3;

                let rgb = if is_height_map {
                    if is_path {
                        // Path color is Rgb::new(0x37, 0x29, 0x23)
                        Rgb::new(0.9, 0.9, 0.63)
                    } else if is_water {
                        Rgb::new(0.23, 0.47, 0.53)
                    } else if is_contours && is_contour_line {
                        // Color contour lines
                        Rgb::new(0.15, 0.15, 0.15)
                    } else {
                        // Color hill shading
                        let lightness = (alt + 0.2).min(1.0);
                        Rgb::new(lightness, 0.9 * lightness, 0.5 * lightness)
                    }
                } else if is_stylized_topo {
                    if is_path {
                        Rgb::new(0.9, 0.9, 0.63)
                    } else if is_water {
                        if is_border {
                            Rgb::new(0.10, 0.34, 0.50)
                        } else {
                            Rgb::new(0.23, 0.47, 0.63)
                        }
                    } else if is_contour_line {
                        Rgb::new(0.25, 0.25, 0.25)
                    } else {
                        // Stylized colors
                        Rgb::new(
                            (rgb.r + 0.25).min(1.0),
                            (rgb.g + 0.23).min(1.0),
                            (rgb.b + 0.10).min(1.0),
                        )
                    }
                } else {
                    Rgb::new(rgb.r, rgb.g, rgb.b)
                }
                .map(|e| (e * 255.0) as u8);
                common::terrain::map::MapSample {
                    rgb,
                    alt,
                    downhill_wpos,
                    connections: None,
                }
            }
            // Generate standard shaded map
            map_config.is_shaded = true;
            map_config.generate(
                |pos| {
                    sample_pos(
                        &map_config,
                        pos,
                        &alt,
                        &rgba,
                        &map_size,
                        &map_size_lg,
                        max_height,
                    )
                },
                |wpos| {
                    let pos = wpos.wpos_to_cpos();
                    rescale_height(if bounds_check(pos) {
                        scale_height_big(alt[pos])
                    } else {
                        0.0
                    })
                },
                |pos, (r, g, b, a)| {
                    world_map_rgba[pos.y * map_size.x as usize + pos.x] =
                        u32::from_le_bytes([r, g, b, a]);
                },
            );
            // Generate map with topographical lines and stylized colors
            map_config.is_contours = true;
            map_config.is_stylized_topo = true;
            map_config.generate(
                |pos| {
                    sample_pos(
                        &map_config,
                        pos,
                        &alt,
                        &rgba,
                        &map_size,
                        &map_size_lg,
                        max_height,
                    )
                },
                |wpos| {
                    let pos = wpos.wpos_to_cpos();
                    rescale_height(if bounds_check(pos) {
                        scale_height_big(alt[pos])
                    } else {
                        0.0
                    })
                },
                |pos, (r, g, b, a)| {
                    world_map_topo[pos.y * map_size.x as usize + pos.x] =
                        u32::from_le_bytes([r, g, b, a]);
                },
            );
            let make_raw = |rgb| -> Result<_, Error> {
                let mut raw = vec![0u8; 4 * world_map_rgba.len()];
                LittleEndian::write_u32_into(rgb, &mut raw);
                Ok(Arc::new(
                    DynamicImage::ImageRgba8({
                        // Should not fail if the dimensions are correct.
                        let map =
                            image::ImageBuffer::from_raw(u32::from(map_size.x), u32::from(map_size.y), raw);
                        map.ok_or_else(|| Error::Other("Server sent a bad world map image".into()))?
                    })
                    // Flip the image, since Voxygen uses an orientation where rotation from
                    // positive x axis to positive y axis is counterclockwise around the z axis.
                    .flipv(),
                ))
            };
            let lod_base = rgba;
            let lod_alt = alt;
            let world_map_rgb_img = make_raw(&world_map_rgba)?;
            let world_map_topo_img = make_raw(&world_map_topo)?;
            let world_map_layers = vec![world_map_rgb_img, world_map_topo_img];
            let horizons = (west.0, west.1, east.0, east.1)
                .into_par_iter()
                .map(|(wa, wh, ea, eh)| u32::from_le_bytes([wa, wh, ea, eh]))
                .collect::<Vec<_>>();
            let lod_horizon = horizons;
            let map_bounds = Vec2::new(sea_level, max_height);
            debug!("Done preparing image...");

            Ok((
                state,
                lod_base,
                lod_alt,
                Grid::from_raw(map_size.map(|e| e as i32), lod_horizon),
                (world_map_layers, map_size, map_bounds),
                world_map.sites,
                world_map.possible_starting_sites,
                world_map.pois,
                component_recipe_book,
                repair_recipe_book,
                max_group_size,
                client_timeout,
                missing_plugins,
                local_plugins,
                role,
            ))
        });

        let (
            state,
            lod_base,
            lod_alt,
            lod_horizon,
            world_map,
            sites,
            possible_starting_sites,
            pois,
            component_recipe_book,
            repair_recipe_book,
            max_group_size,
            client_timeout,
            missing_plugins,
            local_plugins,
            role,
        ) = loop {
            tokio::select! {
                res = &mut task => break res.expect("Client thread should not panic")?,
                _ = ping_interval.tick() => ping_stream.send(PingMsg::Ping)?,
            }
        };
        let missing_plugins_set = missing_plugins.iter().cloned().collect();
        if !missing_plugins.is_empty() {
            stream.send(ClientGeneral::RequestPlugins(missing_plugins))?;
        }
        ping_stream.send(PingMsg::Ping)?;

        debug!("Initial sync done");

        Ok(Self {
            client_type,
            registered: true,
            presence: None,
            runtime,
            server_info,
            server_description: description,
            world_data: WorldData {
                lod_base,
                lod_alt,
                lod_horizon,
                map: world_map,
            },
            weather: WeatherLerp::default(),
            player_list: HashMap::new(),
            character_list: CharacterList::default(),
            character_being_deleted: None,
            sites: sites
                .iter()
                .filter_map(|m| {
                    Some((m.site?, SiteMarker {
                        marker: m.clone(),
                        economy: None,
                    }))
                })
                .collect(),
            extra_markers: sites.iter().filter(|m| m.site.is_none()).cloned().collect(),
            possible_starting_sites,
            pois,
            component_recipe_book,
            repair_recipe_book,
            available_recipes: HashMap::default(),
            chat_mode: ChatMode::default(),

            lod_zones: HashMap::new(),
            lod_last_requested: None,
            lod_pos_fallback: None,

            force_update_counter: 0,

            role,
            max_group_size,
            invite: None,
            group_leader: None,
            group_members: HashMap::new(),
            pending_invites: HashSet::new(),
            pending_trade: None,
            waypoint: None,

            network: Some(network),
            participant: Some(participant),
            general_stream: stream,
            ping_stream,
            register_stream,
            character_screen_stream,
            in_game_stream,
            terrain_stream,

            client_timeout,

            last_server_ping: 0.0,
            last_server_pong: 0.0,
            last_ping_delta: 0.0,
            ping_deltas: VecDeque::new(),

            tick: 0,
            state,

            flashing_lights_enabled: true,

            server_view_distance_limit: None,
            view_distance: None,
            lod_distance: 4.0,
            loaded_distance: 0.0,

            pending_chunks: HashMap::new(),
            target_time_of_day: None,
            dt_adjustment: 1.0,

            connected_server_constants: server_constants,
            missing_plugins: missing_plugins_set,
            local_plugins,
        })
    }

    /// Request a state transition to `ClientState::Registered`.
    async fn register(
        username: &str,
        password: &str,
        locale: Option<String>,
        mut auth_trusted: impl FnMut(&str) -> bool,
        server_info: &ServerInfo,
        register_stream: &mut Stream,
    ) -> Result<(), Error> {
        // Authentication
        let token_or_username = match &server_info.auth_provider {
            Some(addr) => {
                // Query whether this is a trusted auth server
                if auth_trusted(addr) {
                    let (scheme, authority) = match addr.split_once("://") {
                        Some((s, a)) => (s, a),
                        None => return Err(Error::AuthServerUrlInvalid(addr.to_string())),
                    };

                    let scheme = match scheme.parse::<authc::Scheme>() {
                        Ok(s) => s,
                        Err(_) => return Err(Error::AuthServerUrlInvalid(addr.to_string())),
                    };

                    let authority = match authority.parse::<authc::Authority>() {
                        Ok(a) => a,
                        Err(_) => return Err(Error::AuthServerUrlInvalid(addr.to_string())),
                    };

                    Ok(authc::AuthClient::new(scheme, authority)?
                        .sign_in(username, password)
                        .await?
                        .serialize())
                } else {
                    Err(Error::AuthServerNotTrusted)
                }
            },
            None => Ok(username.to_owned()),
        }?;

        debug!("Registering client...");

        register_stream.send(ClientRegister {
            token_or_username,
            locale,
        })?;

        match register_stream.recv::<ServerRegisterAnswer>().await? {
            Err(RegisterError::AuthError(err)) => Err(Error::AuthErr(err)),
            Err(RegisterError::InvalidCharacter) => Err(Error::InvalidCharacter),
            Err(RegisterError::NotOnWhitelist) => Err(Error::NotOnWhitelist),
            Err(RegisterError::Kicked(err)) => Err(Error::Kicked(err)),
            Err(RegisterError::Banned(info)) => Err(Error::Banned(info)),
            Err(RegisterError::TooManyPlayers) => Err(Error::TooManyPlayers),
            Ok(()) => {
                debug!("Client registered successfully.");
                Ok(())
            },
        }
    }

    fn send_msg_err<S>(&mut self, msg: S) -> Result<(), network::StreamError>
    where
        S: Into<ClientMsg>,
    {
        prof_span!("send_msg_err");
        let msg: ClientMsg = msg.into();
        #[cfg(debug_assertions)]
        {
            const C_TYPE: ClientType = ClientType::Game;
            let verified = msg.verify(C_TYPE, self.registered, self.presence);

            // Due to the fact that character loading is performed asynchronously after
            // initial connect it is possible to receive messages after a character load
            // error while in the wrong state.
            if !verified {
                warn!(
                    "Received ClientType::Game message when not in game (Registered: {} Presence: \
                     {:?}), dropping message: {:?} ",
                    self.registered, self.presence, msg
                );
                return Ok(());
            }
        }
        match msg {
            ClientMsg::Type(msg) => self.register_stream.send(msg),
            ClientMsg::Register(msg) => self.register_stream.send(msg),
            ClientMsg::General(msg) => {
                #[cfg(feature = "tracy")]
                let (mut ingame, mut terrain) = (0.0, 0.0);
                let stream = match msg {
                    ClientGeneral::RequestCharacterList
                    | ClientGeneral::CreateCharacter { .. }
                    | ClientGeneral::EditCharacter { .. }
                    | ClientGeneral::DeleteCharacter(_)
                    | ClientGeneral::Character(_, _)
                    | ClientGeneral::Spectate(_) => &mut self.character_screen_stream,
                    // Only in game
                    ClientGeneral::ControllerInputs(_)
                    | ClientGeneral::ControlEvent(_)
                    | ClientGeneral::ControlAction(_)
                    | ClientGeneral::SetViewDistance(_)
                    | ClientGeneral::BreakBlock(_)
                    | ClientGeneral::PlaceBlock(_, _)
                    | ClientGeneral::ExitInGame
                    | ClientGeneral::PlayerPhysics { .. }
                    | ClientGeneral::UnlockSkill(_)
                    | ClientGeneral::RequestSiteInfo(_)
                    | ClientGeneral::RequestPlayerPhysics { .. }
                    | ClientGeneral::RequestLossyTerrainCompression { .. }
                    | ClientGeneral::UpdateMapMarker(_)
                    | ClientGeneral::SpectatePosition(_)
                    | ClientGeneral::SpectateEntity(_)
                    | ClientGeneral::SetBattleMode(_) => {
                        #[cfg(feature = "tracy")]
                        {
                            ingame = 1.0;
                        }
                        &mut self.in_game_stream
                    },
                    // Terrain
                    ClientGeneral::TerrainChunkRequest { .. }
                    | ClientGeneral::LodZoneRequest { .. } => {
                        #[cfg(feature = "tracy")]
                        {
                            terrain = 1.0;
                        }
                        &mut self.terrain_stream
                    },
                    // Always possible
                    ClientGeneral::ChatMsg(_)
                    | ClientGeneral::Command(_, _)
                    | ClientGeneral::Terminate
                    | ClientGeneral::RequestPlugins(_) => &mut self.general_stream,
                };
                #[cfg(feature = "tracy")]
                {
                    plot!("ingame_sends", ingame);
                    plot!("terrain_sends", terrain);
                }
                stream.send(msg)
            },
            ClientMsg::Ping(msg) => self.ping_stream.send(msg),
        }
    }

    pub fn request_player_physics(&mut self, server_authoritative: bool) {
        self.send_msg(ClientGeneral::RequestPlayerPhysics {
            server_authoritative,
        })
    }

    pub fn request_lossy_terrain_compression(&mut self, lossy_terrain_compression: bool) {
        self.send_msg(ClientGeneral::RequestLossyTerrainCompression {
            lossy_terrain_compression,
        })
    }

    fn send_msg<S>(&mut self, msg: S)
    where
        S: Into<ClientMsg>,
    {
        let res = self.send_msg_err(msg);
        if let Err(e) = res {
            warn!(
                ?e,
                "connection to server no longer possible, couldn't send msg"
            );
        }
    }

    /// Request a state transition to `ClientState::Character`.
    pub fn request_character(
        &mut self,
        character_id: CharacterId,
        view_distances: common::ViewDistances,
    ) {
        let view_distances = self.set_view_distances_local(view_distances);
        self.send_msg(ClientGeneral::Character(character_id, view_distances));

        if let Some(character) = self
            .character_list
            .characters
            .iter()
            .find(|x| x.character.id == Some(character_id))
        {
            self.waypoint = character.location.clone();
        }

        // Assume we are in_game unless server tells us otherwise
        self.presence = Some(PresenceKind::Character(character_id));
    }

    /// Request a state transition to `ClientState::Spectate`.
    pub fn request_spectate(&mut self, view_distances: common::ViewDistances) {
        let view_distances = self.set_view_distances_local(view_distances);
        self.send_msg(ClientGeneral::Spectate(view_distances));

        self.presence = Some(PresenceKind::Spectator);
    }

    /// Load the current players character list
    pub fn load_character_list(&mut self) {
        self.character_list.loading = true;
        self.send_msg(ClientGeneral::RequestCharacterList);
    }

    /// New character creation
    pub fn create_character(
        &mut self,
        alias: String,
        mainhand: Option<String>,
        offhand: Option<String>,
        body: comp::Body,
        hardcore: bool,
        start_site: Option<SiteId>,
    ) {
        self.character_list.loading = true;
        self.send_msg(ClientGeneral::CreateCharacter {
            alias,
            mainhand,
            offhand,
            body,
            hardcore,
            start_site,
        });
    }

    pub fn edit_character(&mut self, alias: String, id: CharacterId, body: comp::Body) {
        self.character_list.loading = true;
        self.send_msg(ClientGeneral::EditCharacter { alias, id, body });
    }

    /// Character deletion
    pub fn delete_character(&mut self, character_id: CharacterId) {
        // Pre-emptively remove the character to be deleted from the character list as
        // character deletes are processed asynchronously by the server so we can't rely
        // on a timely response to update the character list
        if let Some(pos) = self
            .character_list
            .characters
            .iter()
            .position(|x| x.character.id == Some(character_id))
        {
            self.character_list.characters.remove(pos);
        }
        self.send_msg(ClientGeneral::DeleteCharacter(character_id));
    }

    /// Send disconnect message to the server
    pub fn logout(&mut self) {
        debug!("Sending logout from server");
        self.send_msg(ClientGeneral::Terminate);
        self.registered = false;
        self.presence = None;
    }

    /// Request a state transition to `ClientState::Registered` from an ingame
    /// state.
    pub fn request_remove_character(&mut self) {
        self.chat_mode = ChatMode::World;
        self.send_msg(ClientGeneral::ExitInGame);
    }

    pub fn set_view_distances(&mut self, view_distances: common::ViewDistances) {
        let view_distances = self.set_view_distances_local(view_distances);
        self.send_msg(ClientGeneral::SetViewDistance(view_distances));
    }

    /// Clamps provided view distances, locally sets the terrain view distance
    /// in the client's properties and returns the clamped values for the
    /// caller to send to the server.
    fn set_view_distances_local(
        &mut self,
        view_distances: common::ViewDistances,
    ) -> common::ViewDistances {
        let view_distances = common::ViewDistances {
            terrain: view_distances
                .terrain
                .clamp(1, MAX_SELECTABLE_VIEW_DISTANCE),
            entity: view_distances.entity.max(1),
        };
        self.view_distance = Some(view_distances.terrain);
        view_distances
    }

    pub fn set_lod_distance(&mut self, lod_distance: u32) {
        let lod_distance = lod_distance.clamp(0, 1000) as f32 / lod::ZONE_SIZE as f32;
        self.lod_distance = lod_distance;
    }

    pub fn set_flashing_lights_enabled(&mut self, flashing_lights_enabled: bool) {
        self.flashing_lights_enabled = flashing_lights_enabled;
    }

    pub fn use_slot(&mut self, slot: Slot) {
        self.control_action(ControlAction::InventoryAction(InventoryAction::Use(slot)))
    }

    pub fn swap_slots(&mut self, a: Slot, b: Slot) {
        match (a, b) {
            (Slot::Overflow(o), Slot::Inventory(inv))
            | (Slot::Inventory(inv), Slot::Overflow(o)) => {
                self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                    InventoryEvent::OverflowMove(o, inv),
                )));
            },
            (Slot::Overflow(_), _) | (_, Slot::Overflow(_)) => {},
            (Slot::Equip(equip), slot) | (slot, Slot::Equip(equip)) => self.control_action(
                ControlAction::InventoryAction(InventoryAction::Swap(equip, slot)),
            ),
            (Slot::Inventory(inv1), Slot::Inventory(inv2)) => {
                self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                    InventoryEvent::Swap(inv1, inv2),
                )))
            },
        }
    }

    pub fn drop_slot(&mut self, slot: Slot) {
        match slot {
            Slot::Equip(equip) => {
                self.control_action(ControlAction::InventoryAction(InventoryAction::Drop(equip)))
            },
            Slot::Inventory(inv) => self.send_msg(ClientGeneral::ControlEvent(
                ControlEvent::InventoryEvent(InventoryEvent::Drop(inv)),
            )),
            Slot::Overflow(o) => self.send_msg(ClientGeneral::ControlEvent(
                ControlEvent::InventoryEvent(InventoryEvent::OverflowDrop(o)),
            )),
        }
    }

    pub fn sort_inventory(&mut self, sort_order: InventorySortOrder) {
        self.control_action(ControlAction::InventoryAction(InventoryAction::Sort(
            sort_order,
        )));
    }

    pub fn perform_trade_action(&mut self, action: TradeAction) {
        if let Some((id, _, _)) = self.pending_trade {
            if let TradeAction::Decline = action {
                self.pending_trade.take();
            }
            self.send_msg(ClientGeneral::ControlEvent(
                ControlEvent::PerformTradeAction(id, action),
            ));
        }
    }

    pub fn is_dead(&self) -> bool { self.current::<comp::Health>().is_some_and(|h| h.is_dead) }

    pub fn is_gliding(&self) -> bool {
        self.current::<CharacterState>()
            .is_some_and(|cs| matches!(cs, CharacterState::Glide(_)))
    }

    pub fn split_swap_slots(&mut self, a: Slot, b: Slot) {
        match (a, b) {
            (Slot::Overflow(_), _) | (_, Slot::Overflow(_)) => {},
            (Slot::Equip(equip), slot) | (slot, Slot::Equip(equip)) => self.control_action(
                ControlAction::InventoryAction(InventoryAction::Swap(equip, slot)),
            ),
            (Slot::Inventory(inv1), Slot::Inventory(inv2)) => {
                self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                    InventoryEvent::SplitSwap(inv1, inv2),
                )))
            },
        }
    }

    pub fn split_drop_slot(&mut self, slot: Slot) {
        match slot {
            Slot::Equip(equip) => {
                self.control_action(ControlAction::InventoryAction(InventoryAction::Drop(equip)))
            },
            Slot::Inventory(inv) => self.send_msg(ClientGeneral::ControlEvent(
                ControlEvent::InventoryEvent(InventoryEvent::SplitDrop(inv)),
            )),
            Slot::Overflow(o) => self.send_msg(ClientGeneral::ControlEvent(
                ControlEvent::InventoryEvent(InventoryEvent::OverflowSplitDrop(o)),
            )),
        }
    }

    pub fn pick_up(&mut self, entity: EcsEntity) {
        // Get the health component from the entity

        if let Some(uid) = self.state.read_component_copied(entity) {
            // If we're dead, exit before sending the message
            if self.is_dead() {
                return;
            }

            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                InventoryEvent::Pickup(uid),
            )));
        }
    }

    pub fn do_pet(&mut self, target_entity: EcsEntity) {
        if self.is_dead() {
            return;
        }

        if let Some(target_uid) = self.state.read_component_copied(target_entity) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InteractWith {
                target: target_uid,
                kind: common::interaction::InteractionKind::Pet,
            }))
        }
    }

    pub fn npc_interact(&mut self, npc_entity: EcsEntity) {
        // If we're dead, exit before sending message
        if self.is_dead() {
            return;
        }

        if let Some(uid) = self.state.read_component_copied(npc_entity) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::Interact(uid)));
        }
    }

    pub fn player_list(&self) -> &HashMap<Uid, PlayerInfo> { &self.player_list }

    pub fn character_list(&self) -> &CharacterList { &self.character_list }

    pub fn server_info(&self) -> &ServerInfo { &self.server_info }

    pub fn server_description(&self) -> &ServerDescription { &self.server_description }

    pub fn world_data(&self) -> &WorldData { &self.world_data }

    pub fn component_recipe_book(&self) -> &ComponentRecipeBook { &self.component_recipe_book }

    pub fn repair_recipe_book(&self) -> &RepairRecipeBook { &self.repair_recipe_book }

    pub fn client_type(&self) -> &ClientType { &self.client_type }

    pub fn available_recipes(&self) -> &HashMap<String, Option<SpriteKind>> {
        &self.available_recipes
    }

    pub fn lod_zones(&self) -> &HashMap<Vec2<i32>, lod::Zone> { &self.lod_zones }

    /// Set the fallback position used for loading LoD zones when the client
    /// entity does not have a position.
    pub fn set_lod_pos_fallback(&mut self, pos: Vec2<f32>) { self.lod_pos_fallback = Some(pos); }

    pub fn craft_recipe(
        &mut self,
        recipe: &str,
        slots: Vec<(u32, InvSlotId)>,
        craft_sprite: Option<(VolumePos, SpriteKind)>,
        amount: u32,
    ) -> bool {
        let (can_craft, has_sprite) = if let Some(inventory) = self
            .state
            .ecs()
            .read_storage::<comp::Inventory>()
            .get(self.entity())
        {
            let rbm = self.state.ecs().read_resource::<RecipeBookManifest>();
            let (can_craft, required_sprite) = inventory.can_craft_recipe(recipe, 1, &rbm);
            let has_sprite =
                required_sprite.is_none_or(|s| Some(s) == craft_sprite.map(|(_, s)| s));
            (can_craft, has_sprite)
        } else {
            (false, false)
        };
        if can_craft && has_sprite {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                InventoryEvent::CraftRecipe {
                    craft_event: CraftEvent::Simple {
                        recipe: recipe.to_string(),
                        slots,
                        amount,
                    },
                    craft_sprite: craft_sprite.map(|(pos, _)| pos),
                },
            )));
            true
        } else {
            false
        }
    }

    /// Checks if the item in the given slot can be salvaged.
    pub fn can_salvage_item(&self, slot: InvSlotId) -> bool {
        self.inventories()
            .get(self.entity())
            .and_then(|inv| inv.get(slot))
            .is_some_and(|item| item.is_salvageable())
    }

    /// Salvage the item in the given inventory slot. `salvage_pos` should be
    /// the location of a relevant crafting station within range of the player.
    pub fn salvage_item(&mut self, slot: InvSlotId, salvage_pos: VolumePos) -> bool {
        let is_salvageable = self.can_salvage_item(slot);
        if is_salvageable {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                InventoryEvent::CraftRecipe {
                    craft_event: CraftEvent::Salvage(slot),
                    craft_sprite: Some(salvage_pos),
                },
            )));
        }
        is_salvageable
    }

    /// Crafts modular weapon from components in the provided slots.
    /// `sprite_pos` should be the location of the necessary crafting station in
    /// range of the player.
    /// Returns whether or not the networking event was sent (which is based on
    /// whether the player has two modular components in the provided slots)
    pub fn craft_modular_weapon(
        &mut self,
        primary_component: InvSlotId,
        secondary_component: InvSlotId,
        sprite_pos: Option<VolumePos>,
    ) -> bool {
        let inventories = self.inventories();
        let inventory = inventories.get(self.entity());

        enum ModKind {
            Primary,
            Secondary,
        }

        // Closure to get inner modular component info from item in a given slot
        let mod_kind = |slot| match inventory
            .and_then(|inv| inv.get(slot).map(|item| item.kind()))
            .as_deref()
        {
            Some(ItemKind::ModularComponent(modular::ModularComponent::ToolPrimaryComponent {
                ..
            })) => Some(ModKind::Primary),
            Some(ItemKind::ModularComponent(
                modular::ModularComponent::ToolSecondaryComponent { .. },
            )) => Some(ModKind::Secondary),
            _ => None,
        };

        if let (Some(ModKind::Primary), Some(ModKind::Secondary)) =
            (mod_kind(primary_component), mod_kind(secondary_component))
        {
            drop(inventories);
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                InventoryEvent::CraftRecipe {
                    craft_event: CraftEvent::ModularWeapon {
                        primary_component,
                        secondary_component,
                    },
                    craft_sprite: sprite_pos,
                },
            )));
            true
        } else {
            false
        }
    }

    pub fn craft_modular_weapon_component(
        &mut self,
        toolkind: tool::ToolKind,
        material: InvSlotId,
        modifier: Option<InvSlotId>,
        slots: Vec<(u32, InvSlotId)>,
        sprite_pos: Option<VolumePos>,
    ) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
            InventoryEvent::CraftRecipe {
                craft_event: CraftEvent::ModularWeaponPrimaryComponent {
                    toolkind,
                    material,
                    modifier,
                    slots,
                },
                craft_sprite: sprite_pos,
            },
        )));
    }

    /// Repairs the item in the given inventory slot. `sprite_pos` should be
    /// the location of a relevant crafting station within range of the player.
    pub fn repair_item(
        &mut self,
        item: Slot,
        slots: Vec<(u32, InvSlotId)>,
        sprite_pos: VolumePos,
    ) -> bool {
        let is_repairable = {
            let inventories = self.inventories();
            let inventory = inventories.get(self.entity());
            inventory.is_some_and(|inv| {
                if let Some(item) = match item {
                    Slot::Equip(equip_slot) => inv.equipped(equip_slot),
                    Slot::Inventory(invslot) => inv.get(invslot),
                    Slot::Overflow(_) => None,
                } {
                    item.has_durability()
                } else {
                    false
                }
            })
        };
        if is_repairable {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InventoryEvent(
                InventoryEvent::CraftRecipe {
                    craft_event: CraftEvent::Repair { item, slots },
                    craft_sprite: Some(sprite_pos),
                },
            )));
        }
        is_repairable
    }

    fn update_available_recipes(&mut self) {
        let rbm = self.state.ecs().read_resource::<RecipeBookManifest>();
        let inventories = self.state.ecs().read_storage::<comp::Inventory>();
        if let Some(inventory) = inventories.get(self.entity()) {
            self.available_recipes = inventory
                .recipes_iter()
                .cloned()
                .filter_map(|name| {
                    let (can_craft, required_sprite) = inventory.can_craft_recipe(&name, 1, &rbm);
                    if can_craft {
                        Some((name, required_sprite))
                    } else {
                        None
                    }
                })
                .collect();
        }
    }

    /// Unstable, likely to be removed in a future release
    pub fn sites(&self) -> &HashMap<SiteId, SiteMarker> { &self.sites }

    pub fn markers(&self) -> impl Iterator<Item = &Marker> {
        self.sites
            .values()
            .map(|s| &s.marker)
            .chain(self.extra_markers.iter())
    }

    pub fn possible_starting_sites(&self) -> &[SiteId] { &self.possible_starting_sites }

    /// Unstable, likely to be removed in a future release
    pub fn pois(&self) -> &Vec<PoiInfo> { &self.pois }

    pub fn enable_lantern(&mut self) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::EnableLantern));
    }

    pub fn disable_lantern(&mut self) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::DisableLantern));
    }

    pub fn toggle_sprite_light(&mut self, pos: VolumePos, enable: bool) {
        self.control_action(ControlAction::InventoryAction(
            InventoryAction::ToggleSpriteLight(pos, enable),
        ));
    }

    pub fn help_downed(&mut self, target_entity: EcsEntity) {
        if self.is_dead() {
            return;
        }

        if let Some(target_uid) = self.state.read_component_copied(target_entity) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InteractWith {
                target: target_uid,
                kind: common::interaction::InteractionKind::HelpDowned,
            }))
        }
    }

    pub fn remove_buff(&mut self, buff_id: BuffKind) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::RemoveBuff(
            buff_id,
        )));
    }

    pub fn leave_stance(&mut self) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::LeaveStance));
    }

    pub fn unlock_skill(&mut self, skill: Skill) {
        self.send_msg(ClientGeneral::UnlockSkill(skill));
    }

    pub fn max_group_size(&self) -> u32 { self.max_group_size }

    pub fn invite(&self) -> Option<(Uid, Instant, Duration, InviteKind)> { self.invite }

    pub fn group_info(&self) -> Option<(String, Uid)> {
        self.group_leader.map(|l| ("Group".into(), l)) // TODO
    }

    pub fn group_members(&self) -> &HashMap<Uid, group::Role> { &self.group_members }

    pub fn pending_invites(&self) -> &HashSet<Uid> { &self.pending_invites }

    pub fn pending_trade(&self) -> &Option<(TradeId, PendingTrade, Option<SitePrices>)> {
        &self.pending_trade
    }

    pub fn is_trading(&self) -> bool { self.pending_trade.is_some() }

    pub fn send_invite(&mut self, invitee: Uid, kind: InviteKind) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InitiateInvite(
            invitee, kind,
        )))
    }

    pub fn accept_invite(&mut self) {
        // Clear invite
        self.invite.take();
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InviteResponse(
            InviteResponse::Accept,
        )));
    }

    pub fn decline_invite(&mut self) {
        // Clear invite
        self.invite.take();
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::InviteResponse(
            InviteResponse::Decline,
        )));
    }

    pub fn leave_group(&mut self) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::GroupManip(
            GroupManip::Leave,
        )));
    }

    pub fn kick_from_group(&mut self, uid: Uid) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::GroupManip(
            GroupManip::Kick(uid),
        )));
    }

    pub fn assign_group_leader(&mut self, uid: Uid) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::GroupManip(
            GroupManip::AssignLeader(uid),
        )));
    }

    pub fn is_riding(&self) -> bool {
        self.state
            .ecs()
            .read_storage::<Is<Rider>>()
            .get(self.entity())
            .is_some()
            || self
                .state
                .ecs()
                .read_storage::<Is<VolumeRider>>()
                .get(self.entity())
                .is_some()
    }

    pub fn is_lantern_enabled(&self) -> bool {
        self.state
            .ecs()
            .read_storage::<comp::LightEmitter>()
            .get(self.entity())
            .is_some()
    }

    pub fn mount(&mut self, entity: EcsEntity) {
        if let Some(uid) = self.state.read_component_copied(entity) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::Mount(uid)));
        }
    }

    /// Mount a block at a `VolumePos`.
    pub fn mount_volume(&mut self, volume_pos: VolumePos) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::MountVolume(
            volume_pos,
        )));
    }

    pub fn unmount(&mut self) { self.send_msg(ClientGeneral::ControlEvent(ControlEvent::Unmount)); }

    pub fn set_pet_stay(&mut self, entity: EcsEntity, stay: bool) {
        if let Some(uid) = self.state.read_component_copied(entity) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::SetPetStay(
                uid, stay,
            )));
        }
    }

    pub fn give_up(&mut self) {
        if comp::is_downed(self.current().as_ref(), self.current().as_ref()) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::GiveUp));
        }
    }

    pub fn respawn(&mut self) -> bool {
        if self.current::<comp::Health>().is_some_and(|h| h.is_dead) {
            // Hardcore characters cannot respawn, kick them to character selection
            if self.current::<Hardcore>().is_some() {
                self.request_remove_character();
            } else {
                self.send_msg(ClientGeneral::ControlEvent(ControlEvent::Respawn));
            }
            true
        } else {
            false
        }
    }

    pub fn map_marker_event(&mut self, event: MapMarkerChange) {
        self.send_msg(ClientGeneral::UpdateMapMarker(event));
    }

    /// Set the current position to spectate, returns true if the client's
    /// player has a Pos component to write to.
    pub fn spectate_position(&mut self, pos: Vec3<f32>) -> bool {
        let write = if let Some(position) = self
            .state
            .ecs()
            .write_storage::<comp::Pos>()
            .get_mut(self.entity())
        {
            position.0 = pos;
            true
        } else {
            false
        };
        if write {
            self.send_msg(ClientGeneral::SpectatePosition(pos));
        }
        write
    }

    pub fn start_spectate_entity(&mut self, entity: EcsEntity) {
        if let Some(uid) = self.state.read_component_copied(entity) {
            self.send_msg(ClientGeneral::SpectateEntity(Some(uid)));
        } else {
            warn!("Spectating entity without a `Uid` component");
        }
    }

    pub fn stop_spectate_entity(&mut self) { self.send_msg(ClientGeneral::SpectateEntity(None)); }

    /// Checks whether a player can swap their weapon+ability `Loadout` settings
    /// and sends the `ControlAction` event that signals to do the swap.
    pub fn swap_loadout(&mut self) { self.control_action(ControlAction::SwapEquippedWeapons) }

    /// Determine whether the player is wielding, if they're even capable of
    /// being in a wield state.
    pub fn is_wielding(&self) -> Option<bool> {
        self.state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(|cs| cs.is_wield())
    }

    pub fn toggle_wield(&mut self) {
        match self.is_wielding() {
            Some(true) => self.control_action(ControlAction::Unwield),
            Some(false) => self.control_action(ControlAction::Wield),
            None => warn!("Can't toggle wield, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn toggle_sit(&mut self) {
        let is_sitting = self
            .state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(|cs| matches!(cs, CharacterState::Sit));

        match is_sitting {
            Some(true) => self.control_action(ControlAction::Stand),
            Some(false) => self.control_action(ControlAction::Sit),
            None => warn!("Can't toggle sit, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn toggle_crawl(&mut self) {
        let is_crawling = self
            .state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(|cs| matches!(cs, CharacterState::Crawl));

        match is_crawling {
            Some(true) => self.control_action(ControlAction::Stand),
            Some(false) => self.control_action(ControlAction::Crawl),
            None => warn!("Can't toggle crawl, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn toggle_dance(&mut self) {
        let is_dancing = self
            .state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(|cs| matches!(cs, CharacterState::Dance));

        match is_dancing {
            Some(true) => self.control_action(ControlAction::Stand),
            Some(false) => self.control_action(ControlAction::Dance),
            None => warn!("Can't toggle dance, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn utter(&mut self, kind: UtteranceKind) {
        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::Utterance(kind)));
    }

    pub fn toggle_sneak(&mut self) {
        let is_sneaking = self
            .state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(CharacterState::is_stealthy);

        match is_sneaking {
            Some(true) => self.control_action(ControlAction::Stand),
            Some(false) => self.control_action(ControlAction::Sneak),
            None => warn!("Can't toggle sneak, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn toggle_glide(&mut self) {
        let using_glider = self
            .state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(|cs| matches!(cs, CharacterState::GlideWield(_) | CharacterState::Glide(_)));

        match using_glider {
            Some(true) => self.control_action(ControlAction::Unwield),
            Some(false) => self.control_action(ControlAction::GlideWield),
            None => warn!("Can't toggle glide, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn cancel_climb(&mut self) {
        let is_climbing = self
            .state
            .ecs()
            .read_storage::<CharacterState>()
            .get(self.entity())
            .map(|cs| matches!(cs, CharacterState::Climb(_)));

        match is_climbing {
            Some(true) => self.control_action(ControlAction::Stand),
            Some(false) => {},
            None => warn!("Can't stop climbing, client entity doesn't have a `CharacterState`"),
        }
    }

    pub fn handle_input(
        &mut self,
        input: InputKind,
        pressed: bool,
        select_pos: Option<Vec3<f32>>,
        target_entity: Option<EcsEntity>,
    ) {
        if pressed {
            self.control_action(ControlAction::StartInput {
                input,
                target_entity: target_entity.and_then(|e| self.state.read_component_copied(e)),
                select_pos,
            });
        } else {
            self.control_action(ControlAction::CancelInput { input });
        }
    }

    pub fn activate_portal(&mut self, portal: EcsEntity) {
        if let Some(portal_uid) = self.state.read_component_copied(portal) {
            self.send_msg(ClientGeneral::ControlEvent(ControlEvent::ActivatePortal(
                portal_uid,
            )));
        }
    }

    fn control_action(&mut self, control_action: ControlAction) {
        if let Some(controller) = self
            .state
            .ecs()
            .write_storage::<Controller>()
            .get_mut(self.entity())
        {
            controller.push_action(control_action);
        }
        self.send_msg(ClientGeneral::ControlAction(control_action));
    }

    fn control_event(&mut self, control_event: ControlEvent) {
        if let Some(controller) = self
            .state
            .ecs()
            .write_storage::<Controller>()
            .get_mut(self.entity())
        {
            controller.push_event(control_event.clone());
        }
        self.send_msg(ClientGeneral::ControlEvent(control_event));
    }

    pub fn view_distance(&self) -> Option<u32> { self.view_distance }

    pub fn server_view_distance_limit(&self) -> Option<u32> { self.server_view_distance_limit }

    pub fn loaded_distance(&self) -> f32 { self.loaded_distance }

    pub fn position(&self) -> Option<Vec3<f32>> {
        self.state
            .read_storage::<comp::Pos>()
            .get(self.entity())
            .map(|v| v.0)
    }

    /// Returns Weather::default if no player position exists.
    pub fn weather_at_player(&self) -> Weather {
        self.position()
            .map(|p| {
                let mut weather = self.state.weather_at(p.xy());
                weather.wind = self.weather.local_wind;
                weather
            })
            .unwrap_or_default()
    }

    pub fn current_chunk(&self) -> Option<Arc<TerrainChunk>> {
        let chunk_pos = Vec2::from(self.position()?)
            .map2(TerrainChunkSize::RECT_SIZE, |e: f32, sz| {
                (e as u32).div_euclid(sz) as i32
            });

        self.state.terrain().get_key_arc(chunk_pos).cloned()
    }

    pub fn current<C>(&self) -> Option<C>
    where
        C: Component + Clone,
    {
        self.state.read_storage::<C>().get(self.entity()).cloned()
    }

    pub fn current_biome(&self) -> BiomeKind {
        match self.current_chunk() {
            Some(chunk) => chunk.meta().biome(),
            _ => BiomeKind::Void,
        }
    }

    pub fn current_site(&self) -> SiteKindMeta {
        let mut player_alt = 0.0;
        if let Some(position) = self.current::<comp::Pos>() {
            player_alt = position.0.z;
        }
        let mut terrain_alt = 0.0;
        let mut site = None;
        if let Some(chunk) = self.current_chunk() {
            terrain_alt = chunk.meta().alt();
            site = chunk.meta().site();
        }
        if player_alt < terrain_alt - 40.0 {
            if let Some(SiteKindMeta::Dungeon(dungeon)) = site {
                SiteKindMeta::Dungeon(dungeon)
            } else {
                SiteKindMeta::Cave
            }
        } else {
            site.unwrap_or_default()
        }
    }

    pub fn request_site_economy(&mut self, id: SiteId) {
        self.send_msg(ClientGeneral::RequestSiteInfo(id))
    }

    pub fn inventories(&self) -> ReadStorage<'_, comp::Inventory> { self.state.read_storage() }

    /// Send a chat message to the server.
    pub fn send_chat(&mut self, message: String) {
        self.send_msg(ClientGeneral::ChatMsg(comp::Content::Plain(message)));
    }

    /// Send a command to the server.
    pub fn send_command(&mut self, name: String, args: Vec<String>) {
        self.send_msg(ClientGeneral::Command(name, args));
    }

    /// Remove all cached terrain
    pub fn clear_terrain(&mut self) {
        self.state.clear_terrain();
        self.pending_chunks.clear();
    }

    pub fn place_block(&mut self, pos: Vec3<i32>, block: Block) {
        self.send_msg(ClientGeneral::PlaceBlock(pos, block));
    }

    pub fn remove_block(&mut self, pos: Vec3<i32>) {
        self.send_msg(ClientGeneral::BreakBlock(pos));
    }

    pub fn collect_block(&mut self, pos: Vec3<i32>) {
        self.control_action(ControlAction::InventoryAction(InventoryAction::Collect(
            pos,
        )));
    }

    pub fn perform_dialogue(&mut self, target: EcsEntity, dialogue: rtsim::Dialogue) {
        if let Some(target_uid) = self.state.read_component_copied(target) {
            // TODO: Add a way to do send-only chat
            // if let Some(msg) = dialogue.message().cloned() {
            //     self.send_msg(ClientGeneral::ChatMsg(msg));
            // }
            self.control_event(ControlEvent::Dialogue(target_uid, dialogue));
        }
    }

    pub fn do_talk(&mut self, tgt: Option<EcsEntity>) {
        if let Some(controller) = self
            .state
            .ecs()
            .write_storage::<comp::Controller>()
            .get_mut(self.entity())
        {
            controller.push_action(ControlAction::Talk(
                tgt.and_then(|tgt| self.state.read_component_copied(tgt)),
            ));
        }
    }

    pub fn change_ability(&mut self, slot: usize, new_ability: comp::ability::AuxiliaryAbility) {
        let auxiliary_key = self
            .inventories()
            .get(self.entity())
            .map_or((None, None), |inv| {
                let tool_kind = |slot| {
                    inv.equipped(slot).and_then(|item| match &*item.kind() {
                        ItemKind::Tool(tool) => Some(tool.kind),
                        _ => None,
                    })
                };

                (
                    tool_kind(EquipSlot::ActiveMainhand),
                    tool_kind(EquipSlot::ActiveOffhand),
                )
            });

        self.send_msg(ClientGeneral::ControlEvent(ControlEvent::ChangeAbility {
            slot,
            auxiliary_key,
            new_ability,
        }))
    }

    pub fn waypoint(&self) -> &Option<String> { &self.waypoint }

    pub fn set_battle_mode(&mut self, battle_mode: BattleMode) {
        self.send_msg(ClientGeneral::SetBattleMode(battle_mode));
    }

    pub fn get_battle_mode(&self) -> BattleMode {
        let Some(uid) = self.uid() else {
            error!("Client entity does not have a Uid component");

            return BattleMode::PvP;
        };

        let Some(player_info) = self.player_list.get(&uid) else {
            error!("Client does not have PlayerInfo for its Uid");

            return BattleMode::PvP;
        };

        let Some(ref character_info) = player_info.character else {
            error!("Client does not have CharacterInfo for its PlayerInfo");

            return BattleMode::PvP;
        };

        character_info.battle_mode
    }

    /// Execute a single client tick, handle input and update the game state by
    /// the given duration.
    pub fn tick(&mut self, inputs: ControllerInputs, dt: Duration) -> Result<Vec<Event>, Error> {
        span!(_guard, "tick", "Client::tick");
        // This tick function is the centre of the Veloren universe. Most client-side
        // things are managed from here, and as such it's important that it
        // stays organised. Please consult the core developers before making
        // significant changes to this code. Here is the approximate order of
        // things. Please update it as this code changes.
        //
        // 1) Collect input from the frontend, apply input effects to the state of the
        //    game
        // 2) Handle messages from the server
        // 3) Go through any events (timer-driven or otherwise) that need handling and
        //    apply them to the state of the game
        // 4) Perform a single LocalState tick (i.e: update the world and entities in
        //    the world)
        // 5) Go through the terrain update queue and apply all changes to the terrain
        // 6) Sync information to the server
        // 7) Finish the tick, passing actions of the main thread back to the frontend

        // 1) Handle input from frontend.
        // Pass character actions from frontend input to the player's entity.
        if self.presence.is_some() {
            prof_span!("handle and send inputs");
            if let Err(e) = self
                .state
                .ecs()
                .write_storage::<Controller>()
                .entry(self.entity())
                .map(|entry| {
                    entry
                        .or_insert_with(|| Controller {
                            inputs: inputs.clone(),
                            queued_inputs: BTreeMap::new(),
                            events: Vec::new(),
                            actions: Vec::new(),
                        })
                        .inputs = inputs.clone();
                })
            {
                let entry = self.entity();
                error!(
                    ?e,
                    ?entry,
                    "Couldn't access controller component on client entity"
                );
            }
            self.send_msg_err(ClientGeneral::ControllerInputs(Box::new(inputs)))?;
        }

        // 2) Build up a list of events for this frame, to be passed to the frontend.
        let mut frontend_events = Vec::new();

        // Prepare for new events
        {
            prof_span!("Last<CharacterState> comps update");
            let ecs = self.state.ecs();
            let mut last_character_states = ecs.write_storage::<comp::Last<CharacterState>>();
            for (entity, _, character_state) in (
                &ecs.entities(),
                &ecs.read_storage::<comp::Body>(),
                &ecs.read_storage::<CharacterState>(),
            )
                .join()
            {
                if let Some(l) = last_character_states
                    .entry(entity)
                    .ok()
                    .map(|l| l.or_insert_with(|| comp::Last(character_state.clone())))
                    // TODO: since this just updates when the variant changes we should
                    // just store the variant to avoid the clone overhead
                    .filter(|l| !character_state.same_variant(&l.0))
                {
                    *l = comp::Last(character_state.clone());
                }
            }
        }

        // Handle new messages from the server.
        frontend_events.append(&mut self.handle_new_messages()?);

        // 3) Update client local data
        // Check if the invite has timed out and remove if so
        if self
            .invite
            .is_some_and(|(_, timeout, dur, _)| timeout.elapsed() > dur)
        {
            self.invite = None;
        }

        // Lerp the clientside weather.
        self.weather.update(&mut self.state.weather_grid_mut());

        if let Some(target_tod) = self.target_time_of_day {
            let mut tod = self.state.ecs_mut().write_resource::<TimeOfDay>();
            tod.0 = target_tod.0;
            self.target_time_of_day = None;
        }

        // Save dead hardcore character ids to avoid displaying in the character list
        // while the server is still in the process of deleting the character
        if self.current::<Hardcore>().is_some()
            && self.is_dead()
            && let Some(PresenceKind::Character(character_id)) = self.presence
        {
            self.character_being_deleted = Some(character_id);
        }

        // 4) Tick the client's LocalState
        self.state.tick(
            Duration::from_secs_f64(dt.as_secs_f64() * self.dt_adjustment),
            true,
            None,
            &self.connected_server_constants,
            |_, _| {},
        );

        // TODO: avoid emitting these in the first place OR actually use outcomes
        // generated locally on the client (if they can be deduplicated from
        // ones that the server generates or if the client can reliably generate
        // them (e.g. syncing skipping character states past certain
        // stages might skip points where outcomes are generated, however we might not
        // care about this?) and the server doesn't need to send them)
        let _ = self.state.ecs().fetch::<EventBus<Outcome>>().recv_all();

        // 5) Terrain
        self.tick_terrain()?;

        // Send a ping to the server once every second
        if self.state.get_program_time() - self.last_server_ping > 1. {
            self.send_msg_err(PingMsg::Ping)?;
            self.last_server_ping = self.state.get_program_time();
        }

        // 6) Update the server about the player's physics attributes.
        if self.presence.is_some()
            && let (Some(pos), Some(vel), Some(ori)) = (
                self.state.read_storage().get(self.entity()).cloned(),
                self.state.read_storage().get(self.entity()).cloned(),
                self.state.read_storage().get(self.entity()).cloned(),
            )
        {
            self.in_game_stream.send(ClientGeneral::PlayerPhysics {
                pos,
                vel,
                ori,
                force_counter: self.force_update_counter,
            })?;
        }

        /*
        // Output debug metrics
        if log_enabled!(Level::Info) && self.tick % 600 == 0 {
            let metrics = self
                .state
                .terrain()
                .iter()
                .fold(ChonkMetrics::default(), |a, (_, c)| a + c.get_metrics());
            info!("{:?}", metrics);
        }
        */

        // 7) Finish the tick, pass control back to the frontend.
        self.tick += 1;
        Ok(frontend_events)
    }

    /// Clean up the client after a tick.
    pub fn cleanup(&mut self) {
        // Cleanup the local state
        self.state.cleanup();
    }

    /// Handles terrain addition and removal.
    ///
    /// Removes old terrain chunks outside the view distance.
    /// Sends requests for missing chunks within the view distance.
    fn tick_terrain(&mut self) -> Result<(), Error> {
        let pos = self
            .state
            .read_storage::<comp::Pos>()
            .get(self.entity())
            .cloned();
        if let (Some(pos), Some(view_distance)) = (pos, self.view_distance) {
            prof_span!("terrain");
            let chunk_pos = self.state.terrain().pos_key(pos.0.map(|e| e as i32));

            // Remove chunks that are too far from the player.
            let mut chunks_to_remove = Vec::new();
            self.state.terrain().iter().for_each(|(key, _)| {
                // Subtract 2 from the offset before computing squared magnitude
                // 1 for the chunks needed bordering other chunks for meshing
                // 1 as a buffer so that if the player moves back in that direction the chunks
                //   don't need to be reloaded
                // Take the minimum of the adjusted difference vs the view_distance + 1 to
                //   prevent magnitude_squared from overflowing

                if (chunk_pos - key)
                    .map(|e: i32| (e.unsigned_abs()).saturating_sub(2).min(view_distance + 1))
                    .magnitude_squared()
                    > view_distance.pow(2)
                {
                    chunks_to_remove.push(key);
                }
            });
            for key in chunks_to_remove {
                self.state.remove_chunk(key);
            }

            let mut current_tick_send_chunk_requests = 0;
            // Request chunks from the server.
            self.loaded_distance = ((view_distance * TerrainChunkSize::RECT_SIZE.x) as f32).powi(2);
            // +1 so we can find a chunk that's outside the vd for better fog
            for dist in 0..view_distance as i32 + 1 {
                // Only iterate through chunks that need to be loaded for circular vd
                // The (dist - 2) explained:
                // -0.5 because a chunk is visible if its corner is within the view distance
                // -0.5 for being able to move to the corner of the current chunk
                // -1 because chunks are not meshed if they don't have all their neighbors
                //     (notice also that view_distance is decreased by 1)
                //     (this subtraction on vd is omitted elsewhere in order to provide
                //     a buffer layer of loaded chunks)
                let top = if 2 * (dist - 2).max(0).pow(2) > (view_distance - 1).pow(2) as i32 {
                    ((view_distance - 1).pow(2) as f32 - (dist - 2).pow(2) as f32)
                        .sqrt()
                        .round() as i32
                        + 1
                } else {
                    dist
                };

                let mut skip_mode = false;
                for i in -top..top + 1 {
                    let keys = [
                        chunk_pos + Vec2::new(dist, i),
                        chunk_pos + Vec2::new(i, dist),
                        chunk_pos + Vec2::new(-dist, i),
                        chunk_pos + Vec2::new(i, -dist),
                    ];

                    for key in keys.iter() {
                        let dist_to_player = (TerrainGrid::key_chunk(*key).map(|x| x as f32)
                            + TerrainChunkSize::RECT_SIZE.map(|x| x as f32) / 2.0)
                            .distance_squared(pos.0.into());

                        let terrain = self.state.terrain();
                        if let Some(chunk) = terrain.get_key_arc(*key) {
                            if !skip_mode && !terrain.contains_key_real(*key) {
                                let chunk = Arc::clone(chunk);
                                drop(terrain);
                                self.state.insert_chunk(*key, chunk);
                            }
                        } else {
                            drop(terrain);
                            if !skip_mode && !self.pending_chunks.contains_key(key) {
                                const TOTAL_PENDING_CHUNKS_LIMIT: usize = 12;
                                const CURRENT_TICK_PENDING_CHUNKS_LIMIT: usize = 2;
                                if self.pending_chunks.len() < TOTAL_PENDING_CHUNKS_LIMIT
                                    && current_tick_send_chunk_requests
                                        < CURRENT_TICK_PENDING_CHUNKS_LIMIT
                                {
                                    self.send_msg_err(ClientGeneral::TerrainChunkRequest {
                                        key: *key,
                                    })?;
                                    current_tick_send_chunk_requests += 1;
                                    self.pending_chunks.insert(*key, Instant::now());
                                } else {
                                    skip_mode = true;
                                }
                            }

                            if dist_to_player < self.loaded_distance {
                                self.loaded_distance = dist_to_player;
                            }
                        }
                    }
                }
            }
            self.loaded_distance = self.loaded_distance.sqrt()
                - ((TerrainChunkSize::RECT_SIZE.x as f32 / 2.0).powi(2)
                    + (TerrainChunkSize::RECT_SIZE.y as f32 / 2.0).powi(2))
                .sqrt();

            // If chunks are taking too long, assume they're no longer pending.
            let now = Instant::now();
            self.pending_chunks
                .retain(|_, created| now.duration_since(*created) < Duration::from_secs(3));
        }

        if let Some(lod_pos) = pos.map(|p| p.0.xy()).or(self.lod_pos_fallback) {
            // Manage LoD zones
            let lod_zone = lod_pos.map(|e| lod::from_wpos(e as i32));

            // Request LoD zones that are in range
            if self
                .lod_last_requested
                .is_none_or(|i| i.elapsed() > Duration::from_secs(5))
                && let Some(rpos) = Spiral2d::new()
                    .take((1 + self.lod_distance.ceil() as i32 * 2).pow(2) as usize)
                    .filter(|rpos| !self.lod_zones.contains_key(&(lod_zone + *rpos)))
                    .min_by_key(|rpos| rpos.magnitude_squared())
                    .filter(|rpos| {
                        rpos.map(|e| e as f32).magnitude() < (self.lod_distance - 0.5).max(0.0)
                    })
            {
                self.send_msg_err(ClientGeneral::LodZoneRequest {
                    key: lod_zone + rpos,
                })?;
                self.lod_last_requested = Some(Instant::now());
            }

            // Cull LoD zones out of range
            self.lod_zones.retain(|p, _| {
                (*p - lod_zone).map(|e| e as f32).magnitude_squared() < self.lod_distance.powi(2)
            });
        }

        Ok(())
    }

    fn handle_server_msg(
        &mut self,
        frontend_events: &mut Vec<Event>,
        msg: ServerGeneral,
    ) -> Result<(), Error> {
        prof_span!("handle_server_msg");
        match msg {
            ServerGeneral::Disconnect(reason) => match reason {
                DisconnectReason::Shutdown => return Err(Error::ServerShutdown),
                DisconnectReason::Kicked(reason) => return Err(Error::Kicked(reason)),
                DisconnectReason::Banned(info) => return Err(Error::Banned(info)),
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::Init(list)) => {
                self.player_list = list
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::Add(uid, player_info)) => {
                if let Some(old_player_info) = self.player_list.insert(uid, player_info.clone()) {
                    warn!(
                        "Received msg to insert {} with uid {} into the player list but there was \
                         already an entry for {} with the same uid that was overwritten!",
                        player_info.player_alias, uid, old_player_info.player_alias
                    );
                }
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::Moderator(uid, moderator)) => {
                if let Some(player_info) = self.player_list.get_mut(&uid) {
                    player_info.is_moderator = moderator;
                } else {
                    warn!(
                        "Received msg to update admin status of uid {}, but they were not in the \
                         list.",
                        uid
                    );
                }
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::SelectedCharacter(
                uid,
                char_info,
            )) => {
                if let Some(player_info) = self.player_list.get_mut(&uid) {
                    player_info.character = Some(char_info);
                } else {
                    warn!(
                        "Received msg to update character info for uid {}, but they were not in \
                         the list.",
                        uid
                    );
                }
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::ExitCharacter(uid)) => {
                if let Some(player_info) = self.player_list.get_mut(&uid) {
                    if player_info.character.is_none() {
                        debug!(?player_info.player_alias, ?uid, "Received PlayerListUpdate::ExitCharacter for a player who wasnt ingame");
                    }
                    player_info.character = None;
                } else {
                    debug!(
                        ?uid,
                        "Received PlayerListUpdate::ExitCharacter for a nonexitent player"
                    );
                }
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::Remove(uid)) => {
                // Instead of removing players, mark them as offline because we need to
                // remember the names of disconnected players in chat.
                //
                // TODO: consider alternatives since this leads to an ever growing list as
                // players log out and in. Keep in mind we might only want to
                // keep only so many messages in chat the history. We could
                // potentially use an ID that's more persistent than the Uid.
                // One of the reasons we don't just store the string of the player name
                // into the message is to make alias changes reflected in older messages.

                if let Some(player_info) = self.player_list.get_mut(&uid) {
                    if player_info.is_online {
                        player_info.is_online = false;
                    } else {
                        warn!(
                            "Received msg to remove uid {} from the player list by they were \
                             already marked offline",
                            uid
                        );
                    }
                } else {
                    warn!(
                        "Received msg to remove uid {} from the player list by they weren't in \
                         the list!",
                        uid
                    );
                }
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::Alias(uid, new_name)) => {
                if let Some(player_info) = self.player_list.get_mut(&uid) {
                    player_info.player_alias = new_name;
                } else {
                    warn!(
                        "Received msg to alias player with uid {} to {} but this uid is not in \
                         the player list",
                        uid, new_name
                    );
                }
            },
            ServerGeneral::PlayerListUpdate(PlayerListUpdate::UpdateBattleMode(
                uid,
                battle_mode,
            )) => {
                if let Some(player_info) = self.player_list.get_mut(&uid) {
                    if let Some(ref mut character_info) = player_info.character {
                        character_info.battle_mode = battle_mode;
                    } else {
                        warn!(
                            "Received msg to update battle mode of uid {} to {:?} but this player \
                             does not have a character",
                            uid, battle_mode
                        );
                    }
                } else {
                    warn!(
                        "Received msg to update battle mode of uid {} to {:?} but this uid is not \
                         in the player list",
                        uid, battle_mode
                    );
                }
            },
            ServerGeneral::ChatMsg(m) => frontend_events.push(Event::Chat(m)),
            ServerGeneral::ChatMode(m) => {
                self.chat_mode = m;
            },
            ServerGeneral::SetPlayerEntity(uid) => {
                if let Some(entity) = self.state.ecs().entity_from_uid(uid) {
                    let old_player_entity = mem::replace(
                        &mut *self.state.ecs_mut().write_resource(),
                        PlayerEntity(Some(entity)),
                    );
                    if let Some(old_entity) = old_player_entity.0 {
                        // Transfer controller to the new entity.
                        let mut controllers = self.state.ecs().write_storage::<Controller>();
                        if let Some(controller) = controllers.remove(old_entity)
                            && let Err(e) = controllers.insert(entity, controller)
                        {
                            error!(
                                ?e,
                                "Failed to insert controller when setting new player entity!"
                            );
                        }
                    }
                    if let Some(presence) = self.presence {
                        self.presence = Some(match presence {
                            PresenceKind::Spectator => PresenceKind::Spectator,
                            PresenceKind::LoadingCharacter(_) => PresenceKind::Possessor,
                            PresenceKind::Character(_) => PresenceKind::Possessor,
                            PresenceKind::Possessor => PresenceKind::Possessor,
                        });
                    }
                    // Clear pending trade
                    self.pending_trade = None;
                } else {
                    return Err(Error::Other("Failed to find entity from uid.".into()));
                }
            },
            ServerGeneral::TimeOfDay(time_of_day, calendar, new_time, time_scale) => {
                self.target_time_of_day = Some(time_of_day);
                *self.state.ecs_mut().write_resource() = calendar;
                *self.state.ecs_mut().write_resource() = time_scale;
                let mut time = self.state.ecs_mut().write_resource::<Time>();
                // Avoid side-eye from Einstein
                // If new time from server is at least 5 seconds ahead, replace client time.
                // Otherwise try to slightly twean client time (by 1%) to keep it in line with
                // server time.
                self.dt_adjustment = if new_time.0 > time.0 + 5.0 {
                    *time = new_time;
                    1.0
                } else if new_time.0 > time.0 {
                    1.01
                } else {
                    0.99
                };
            },
            ServerGeneral::EntitySync(entity_sync_package) => {
                let uid = self.uid();
                self.state
                    .ecs_mut()
                    .apply_entity_sync_package(entity_sync_package, uid);
            },
            ServerGeneral::CompSync(comp_sync_package, force_counter) => {
                self.force_update_counter = force_counter;
                self.state
                    .ecs_mut()
                    .apply_comp_sync_package(comp_sync_package);
            },
            ServerGeneral::CreateEntity(entity_package) => {
                self.state.ecs_mut().apply_entity_package(entity_package);
            },
            ServerGeneral::DeleteEntity(entity_uid) => {
                if self.uid() != Some(entity_uid) {
                    self.state
                        .ecs_mut()
                        .delete_entity_and_clear_uid_mapping(entity_uid);
                }
            },
            ServerGeneral::Notification(n) => {
                let Notification::WaypointSaved { location_name } = n.clone();
                self.waypoint = Some(location_name);

                frontend_events.push(Event::Notification(UserNotification::WaypointUpdated));
            },
            ServerGeneral::PluginData(d) => {
                let plugin_len = d.len();
                tracing::info!(?plugin_len, "plugin data");
                frontend_events.push(Event::PluginDataReceived(d));
            },
            ServerGeneral::SetPlayerRole(role) => {
                debug!(?role, "Updating client role");
                self.role = role;
            },
            _ => unreachable!("Not a general msg"),
        }
        Ok(())
    }

    fn handle_server_in_game_msg(
        &mut self,
        frontend_events: &mut Vec<Event>,
        msg: ServerGeneral,
    ) -> Result<(), Error> {
        prof_span!("handle_server_in_game_msg");
        match msg {
            ServerGeneral::GroupUpdate(change_notification) => {
                use comp::group::ChangeNotification::*;
                // Note: we use a hashmap since this would not work with entities outside
                // the view distance
                match change_notification {
                    Added(uid, role) => {
                        // Check if this is a newly formed group by looking for absence of
                        // other non pet group members
                        if !matches!(role, group::Role::Pet)
                            && !self
                                .group_members
                                .values()
                                .any(|r| !matches!(r, group::Role::Pet))
                        {
                            frontend_events
                                // TODO: localise
                                .push(Event::Chat(comp::ChatType::Meta.into_plain_msg(
                                    "Type /g or /group to chat with your group members",
                                )));
                        }
                        if let Some(player_info) = self.player_list.get(&uid) {
                            frontend_events.push(Event::Chat(
                                // TODO: localise, uses deprecated personalize_alias
                                #[expect(deprecated, reason = "i18n alias")]
                                comp::ChatType::GroupMeta("Group".into()).into_plain_msg(format!(
                                    "[{}] joined group",
                                    self.personalize_alias(uid, player_info.player_alias.clone())
                                )),
                            ));
                        }
                        if self.group_members.insert(uid, role) == Some(role) {
                            warn!(
                                "Received msg to add uid {} to the group members but they were \
                                 already there",
                                uid
                            );
                        }
                    },
                    Removed(uid) => {
                        if let Some(player_info) = self.player_list.get(&uid) {
                            frontend_events.push(Event::Chat(
                                // TODO: localise, uses deprecated personalize_alias
                                #[expect(deprecated, reason = "i18n alias")]
                                comp::ChatType::GroupMeta("Group".into()).into_plain_msg(format!(
                                    "[{}] left group",
                                    self.personalize_alias(uid, player_info.player_alias.clone())
                                )),
                            ));
                            frontend_events.push(Event::MapMarker(
                                comp::MapMarkerUpdate::GroupMember(uid, MapMarkerChange::Remove),
                            ));
                        }
                        if self.group_members.remove(&uid).is_none() {
                            warn!(
                                "Received msg to remove uid {} from group members but by they \
                                 weren't in there!",
                                uid
                            );
                        }
                    },
                    NewLeader(leader) => {
                        self.group_leader = Some(leader);
                    },
                    NewGroup { leader, members } => {
                        self.group_leader = Some(leader);
                        self.group_members = members.into_iter().collect();
                        // Currently add/remove messages treat client as an implicit member
                        // of the group whereas this message explicitly includes them so to
                        // be consistent for now we will remove the client from the
                        // received hashset
                        if let Some(uid) = self.uid() {
                            self.group_members.remove(&uid);
                        }
                        frontend_events.push(Event::MapMarker(comp::MapMarkerUpdate::ClearGroup));
                    },
                    NoGroup => {
                        self.group_leader = None;
                        self.group_members = HashMap::new();
                        frontend_events.push(Event::MapMarker(comp::MapMarkerUpdate::ClearGroup));
                    },
                }
            },
            ServerGeneral::Invite {
                inviter,
                timeout,
                kind,
            } => {
                self.invite = Some((inviter, Instant::now(), timeout, kind));
            },
            ServerGeneral::InvitePending(uid) => {
                if !self.pending_invites.insert(uid) {
                    warn!("Received message about pending invite that was already pending");
                }
            },
            ServerGeneral::InviteComplete {
                target,
                answer,
                kind,
            } => {
                if !self.pending_invites.remove(&target) {
                    warn!(
                        "Received completed invite message for invite that was not in the list of \
                         pending invites"
                    )
                }
                frontend_events.push(Event::InviteComplete {
                    target,
                    answer,
                    kind,
                });
            },
            ServerGeneral::GroupInventoryUpdate(item, uid) => {
                frontend_events.push(Event::GroupInventoryUpdate(item, uid));
            },
            // Cleanup for when the client goes back to the `presence = None`
            ServerGeneral::ExitInGameSuccess => {
                self.presence = None;
                self.clean_state();
            },
            ServerGeneral::InventoryUpdate(inventory, events) => {
                let mut update_inventory = false;
                for event in events.iter() {
                    match event {
                        InventoryUpdateEvent::BlockCollectFailed { .. } => {},
                        InventoryUpdateEvent::EntityCollectFailed { .. } => {},
                        _ => update_inventory = true,
                    }
                }
                if update_inventory {
                    // Push the updated inventory component to the client
                    // FIXME: Figure out whether this error can happen under normal gameplay,
                    // if not find a better way to handle it, if so maybe consider kicking the
                    // client back to login?
                    let entity = self.entity();
                    if let Err(e) = self
                        .state
                        .ecs_mut()
                        .write_storage()
                        .insert(entity, inventory)
                    {
                        warn!(
                            ?e,
                            "Received an inventory update event for client entity, but this \
                             entity was not found... this may be a bug."
                        );
                    }
                }

                self.update_available_recipes();

                frontend_events.push(Event::InventoryUpdated(events));
            },
            ServerGeneral::Dialogue(sender, dialogue) => {
                frontend_events.push(Event::Dialogue(sender, dialogue));
            },
            ServerGeneral::SetViewDistance(vd) => {
                self.view_distance = Some(vd);
                frontend_events.push(Event::SetViewDistance(vd));
                // If the server is correcting client vd selection we assume this is the max
                // allowed view distance.
                self.server_view_distance_limit = Some(vd);
            },
            ServerGeneral::Outcomes(outcomes) => {
                frontend_events.extend(outcomes.into_iter().map(Event::Outcome))
            },
            ServerGeneral::Knockback(impulse) => {
                self.state
                    .ecs()
                    .read_resource::<EventBus<LocalEvent>>()
                    .emit_now(LocalEvent::ApplyImpulse {
                        entity: self.entity(),
                        impulse,
                    });
            },
            ServerGeneral::UpdatePendingTrade(id, trade, pricing) => {
                trace!("UpdatePendingTrade {:?} {:?}", id, trade);
                self.pending_trade = Some((id, trade, pricing));
            },
            ServerGeneral::FinishedTrade(result) => {
                if let Some((_, trade, _)) = self.pending_trade.take() {
                    self.update_available_recipes();
                    frontend_events.push(Event::TradeComplete { result, trade })
                }
            },
            ServerGeneral::SiteEconomy(economy) => {
                if let Some(rich) = self.sites.get_mut(&economy.id) {
                    rich.economy = Some(economy);
                }
            },
            ServerGeneral::MapMarker(event) => {
                frontend_events.push(Event::MapMarker(event));
            },
            ServerGeneral::WeatherUpdate(weather) => {
                self.weather.weather_update(weather);
            },
            ServerGeneral::LocalWindUpdate(wind) => {
                self.weather.local_wind_update(wind);
            },
            ServerGeneral::SpectatePosition(pos) => {
                frontend_events.push(Event::SpectatePosition(pos));
            },
            ServerGeneral::UpdateRecipes => {
                self.update_available_recipes();
            },
            ServerGeneral::Gizmos(gizmos) => frontend_events.push(Event::Gizmos(gizmos)),
            _ => unreachable!("Not a in_game message"),
        }
        Ok(())
    }

    fn handle_server_terrain_msg(&mut self, msg: ServerGeneral) -> Result<(), Error> {
        prof_span!("handle_server_terrain_mgs");
        match msg {
            ServerGeneral::TerrainChunkUpdate { key, chunk } => {
                if let Some(chunk) = chunk.ok().and_then(|c| c.to_chunk()) {
                    self.state.insert_chunk(key, Arc::new(chunk));
                }
                self.pending_chunks.remove(&key);
            },
            ServerGeneral::LodZoneUpdate { key, zone } => {
                self.lod_zones.insert(key, zone);
                self.lod_last_requested = None;
            },
            ServerGeneral::TerrainBlockUpdates(blocks) => {
                if let Some(mut blocks) = blocks.decompress() {
                    blocks.drain().for_each(|(pos, block)| {
                        self.state.set_block(pos, block);
                    });
                }
            },
            _ => unreachable!("Not a terrain message"),
        }
        Ok(())
    }

    fn handle_server_character_screen_msg(
        &mut self,
        events: &mut Vec<Event>,
        msg: ServerGeneral,
    ) -> Result<(), Error> {
        prof_span!("handle_server_character_screen_msg");
        match msg {
            ServerGeneral::CharacterListUpdate(character_list) => {
                self.character_list.characters = character_list;
                if self.character_being_deleted.is_some() {
                    if let Some(pos) = self
                        .character_list
                        .characters
                        .iter()
                        .position(|x| x.character.id == self.character_being_deleted)
                    {
                        self.character_list.characters.remove(pos);
                    } else {
                        self.character_being_deleted = None;
                    }
                }
                self.character_list.loading = false;
            },
            ServerGeneral::CharacterActionError(error) => {
                warn!("CharacterActionError: {:?}.", error);
                events.push(Event::CharacterError(error));
            },
            ServerGeneral::CharacterDataLoadResult(Ok(metadata)) => {
                trace!("Handling join result by server");
                events.push(Event::CharacterJoined(metadata));
            },
            ServerGeneral::CharacterDataLoadResult(Err(error)) => {
                trace!("Handling join error by server");
                self.presence = None;
                self.clean_state();
                events.push(Event::CharacterError(error));
            },
            ServerGeneral::CharacterCreated(character_id) => {
                events.push(Event::CharacterCreated(character_id));
            },
            ServerGeneral::CharacterEdited(character_id) => {
                events.push(Event::CharacterEdited(character_id));
            },
            ServerGeneral::CharacterSuccess => debug!("client is now in ingame state on server"),
            ServerGeneral::SpectatorSuccess(spawn_point) => {
                events.push(Event::StartSpectate(spawn_point));
                debug!("client is now in ingame state on server");
            },
            _ => unreachable!("Not a character_screen msg"),
        }
        Ok(())
    }

    fn handle_ping_msg(&mut self, msg: PingMsg) -> Result<(), Error> {
        prof_span!("handle_ping_msg");
        match msg {
            PingMsg::Ping => {
                self.send_msg_err(PingMsg::Pong)?;
            },
            PingMsg::Pong => {
                self.last_server_pong = self.state.get_program_time();
                self.last_ping_delta = self.state.get_program_time() - self.last_server_ping;

                // Maintain the correct number of deltas for calculating the rolling average
                // ping. The client sends a ping to the server every second so we should be
                // receiving a pong reply roughly every second.
                while self.ping_deltas.len() > PING_ROLLING_AVERAGE_SECS - 1 {
                    self.ping_deltas.pop_front();
                }
                self.ping_deltas.push_back(self.last_ping_delta);
            },
        }
        Ok(())
    }

    fn handle_messages(&mut self, frontend_events: &mut Vec<Event>) -> Result<u64, Error> {
        let mut cnt = 0;
        #[cfg(feature = "tracy")]
        let (mut terrain_cnt, mut ingame_cnt) = (0, 0);
        loop {
            let cnt_start = cnt;

            while let Some(msg) = self.general_stream.try_recv()? {
                cnt += 1;
                self.handle_server_msg(frontend_events, msg)?;
            }
            while let Some(msg) = self.ping_stream.try_recv()? {
                cnt += 1;
                self.handle_ping_msg(msg)?;
            }
            while let Some(msg) = self.character_screen_stream.try_recv()? {
                cnt += 1;
                self.handle_server_character_screen_msg(frontend_events, msg)?;
            }
            while let Some(msg) = self.in_game_stream.try_recv()? {
                cnt += 1;
                #[cfg(feature = "tracy")]
                {
                    ingame_cnt += 1;
                }
                self.handle_server_in_game_msg(frontend_events, msg)?;
            }
            while let Some(msg) = self.terrain_stream.try_recv()? {
                cnt += 1;
                #[cfg(feature = "tracy")]
                {
                    if let ServerGeneral::TerrainChunkUpdate { chunk, .. } = &msg {
                        terrain_cnt += chunk.as_ref().map(|x| x.approx_len()).unwrap_or(0);
                    }
                }
                self.handle_server_terrain_msg(msg)?;
            }

            if cnt_start == cnt {
                #[cfg(feature = "tracy")]
                {
                    plot!("terrain_recvs", terrain_cnt as f64);
                    plot!("ingame_recvs", ingame_cnt as f64);
                }
                return Ok(cnt);
            }
        }
    }

    /// Handle new server messages.
    fn handle_new_messages(&mut self) -> Result<Vec<Event>, Error> {
        prof_span!("handle_new_messages");
        let mut frontend_events = Vec::new();

        // Check that we have an valid connection.
        // Use the last ping time as a 1s rate limiter, we only notify the user once per
        // second
        if self.state.get_program_time() - self.last_server_ping > 1. {
            let duration_since_last_pong = self.state.get_program_time() - self.last_server_pong;

            // Dispatch a notification to the HUD warning they will be kicked in {n} seconds
            const KICK_WARNING_AFTER_REL_TO_TIMEOUT_FRACTION: f64 = 0.75;
            if duration_since_last_pong
                >= (self.client_timeout.as_secs() as f64
                    * KICK_WARNING_AFTER_REL_TO_TIMEOUT_FRACTION)
                && self.state.get_program_time() - duration_since_last_pong > 0.
            {
                frontend_events.push(Event::DisconnectionNotification(
                    (self.state.get_program_time() - duration_since_last_pong).round() as u64,
                ));
            }
        }

        let msg_count = self.handle_messages(&mut frontend_events)?;

        if msg_count == 0
            && self.state.get_program_time() - self.last_server_pong
                > self.client_timeout.as_secs() as f64
        {
            return Err(Error::ServerTimeout);
        }

        // ignore network events
        while let Some(res) = self
            .participant
            .as_mut()
            .and_then(|p| p.try_fetch_event().transpose())
        {
            let event = res?;
            trace!(?event, "received network event");
        }

        Ok(frontend_events)
    }

    pub fn entity(&self) -> EcsEntity {
        self.state
            .ecs()
            .read_resource::<PlayerEntity>()
            .0
            .expect("Client::entity should always have PlayerEntity be Some")
    }

    pub fn uid(&self) -> Option<Uid> { self.state.read_component_copied(self.entity()) }

    pub fn presence(&self) -> Option<PresenceKind> { self.presence }

    pub fn registered(&self) -> bool { self.registered }

    pub fn get_tick(&self) -> u64 { self.tick }

    pub fn get_ping_ms(&self) -> f64 { self.last_ping_delta * 1000.0 }

    pub fn get_ping_ms_rolling_avg(&self) -> f64 {
        let mut total_weight = 0.;
        let pings = self.ping_deltas.len() as f64;
        (self
            .ping_deltas
            .iter()
            .enumerate()
            .fold(0., |acc, (i, ping)| {
                let weight = i as f64 + 1. / pings;
                total_weight += weight;
                acc + (weight * ping)
            })
            / total_weight)
            * 1000.0
    }

    /// Get a reference to the client's runtime thread pool. This pool should be
    /// used for any computationally expensive operations that run outside
    /// of the main thread (i.e., threads that block on I/O operations are
    /// exempt).
    pub fn runtime(&self) -> &Arc<Runtime> { &self.runtime }

    /// Get a reference to the client's game state.
    pub fn state(&self) -> &State { &self.state }

    /// Get a mutable reference to the client's game state.
    pub fn state_mut(&mut self) -> &mut State { &mut self.state }

    /// Returns an iterator over the aliases of all the online players on the
    /// server
    pub fn players(&self) -> impl Iterator<Item = &str> {
        self.player_list()
            .values()
            .filter_map(|player_info| player_info.is_online.then_some(&*player_info.player_alias))
    }

    /// Return true if this client is a moderator on the server
    pub fn is_moderator(&self) -> bool { self.role.is_some() }

    pub fn role(&self) -> &Option<AdminRole> { &self.role }

    /// Clean client ECS state
    fn clean_state(&mut self) {
        // Clear pending trade
        self.pending_trade = None;

        let client_uid = self.uid().expect("Client doesn't have a Uid!!!");

        // Clear ecs of all entities
        self.state.ecs_mut().delete_all();
        self.state.ecs_mut().maintain();
        self.state.ecs_mut().insert(IdMaps::default());

        // Recreate client entity with Uid
        let entity_builder = self.state.ecs_mut().create_entity();
        entity_builder
            .world
            .write_resource::<IdMaps>()
            .add_entity(client_uid, entity_builder.entity);

        let entity = entity_builder.with(client_uid).build();
        self.state.ecs().write_resource::<PlayerEntity>().0 = Some(entity);
    }

    /// Change player alias to "You" if client belongs to matching player
    // TODO: move this to voxygen or i18n-helpers and properly localize there
    // or what's better, just remove completely, it won't properly work with
    // localization anyway.
    #[deprecated = "this function doesn't localize"]
    fn personalize_alias(&self, uid: Uid, alias: String) -> String {
        let client_uid = self.uid().expect("Client doesn't have a Uid!!!");
        if client_uid == uid {
            "You".to_string()
        } else {
            alias
        }
    }

    /// Get important information from client that is necessary for message
    /// localisation
    pub fn lookup_msg_context(&self, msg: &comp::ChatMsg) -> ChatTypeContext {
        let mut result = ChatTypeContext {
            you: self.uid().expect("Client doesn't have a Uid!!!"),
            player_info: HashMap::new(),
            entity_name: HashMap::new(),
        };

        let name_of_uid = |uid| {
            let ecs = self.state().ecs();
            let id_maps = ecs.read_resource::<common::uid::IdMaps>();
            id_maps.uid_entity(uid).and_then(|e| {
                ecs.read_storage::<comp::Stats>()
                    .get(e)
                    .map(|s| s.name.clone())
            })
        };

        let mut add_data_of = |uid| {
            match self.player_list.get(uid) {
                Some(player_info) => {
                    result.player_info.insert(*uid, player_info.clone());
                },
                None => {
                    result.entity_name.insert(
                        *uid,
                        name_of_uid(*uid).unwrap_or_else(|| Content::Plain("<?>".to_string())),
                    );
                },
            };
        };

        match &msg.chat_type {
            comp::ChatType::Online(uid) | comp::ChatType::Offline(uid) => add_data_of(uid),
            comp::ChatType::Kill(kill_source, victim) => {
                add_data_of(victim);

                match kill_source {
                    KillSource::Player(attacker_uid, _) => {
                        add_data_of(attacker_uid);
                    },
                    KillSource::NonPlayer(_, _)
                    | KillSource::FallDamage
                    | KillSource::Suicide
                    | KillSource::NonExistent(_)
                    | KillSource::Other => (),
                };
            },
            comp::ChatType::Tell(from, to) | comp::ChatType::NpcTell(from, to) => {
                add_data_of(from);
                add_data_of(to);
            },
            comp::ChatType::Say(uid)
            | comp::ChatType::Region(uid)
            | comp::ChatType::World(uid)
            | comp::ChatType::NpcSay(uid)
            | comp::ChatType::Group(uid, _)
            | comp::ChatType::Faction(uid, _)
            | comp::ChatType::Npc(uid) => add_data_of(uid),
            comp::ChatType::CommandError
            | comp::ChatType::CommandInfo
            | comp::ChatType::FactionMeta(_)
            | comp::ChatType::GroupMeta(_)
            | comp::ChatType::Meta => (),
        };
        result
    }

    /// Execute a single client tick:
    /// - handles messages from the server
    /// - sends physics update
    /// - requests chunks
    ///
    /// The game state is purposefully not simulated to reduce the overhead of
    /// running the client. This method is for use in testing a server with
    /// many clients connected.
    #[cfg(feature = "tick_network")]
    #[expect(clippy::needless_collect)] // False positive
    pub fn tick_network(&mut self, dt: Duration) -> Result<(), Error> {
        span!(_guard, "tick_network", "Client::tick_network");
        // Advance state time manually since we aren't calling `State::tick`
        self.state
            .ecs()
            .write_resource::<common::resources::ProgramTime>()
            .0 += dt.as_secs_f64();

        let time_scale = *self
            .state
            .ecs()
            .read_resource::<common::resources::TimeScale>();
        self.state
            .ecs()
            .write_resource::<common::resources::Time>()
            .0 += dt.as_secs_f64() * time_scale.0;

        // Handle new messages from the server.
        self.handle_new_messages()?;

        // 5) Terrain
        self.tick_terrain()?;
        let empty = Arc::new(TerrainChunk::new(
            0,
            Block::empty(),
            Block::empty(),
            common::terrain::TerrainChunkMeta::void(),
        ));
        let mut terrain = self.state.terrain_mut();
        // Replace chunks with empty chunks to save memory
        let to_clear = terrain
            .iter()
            .filter_map(|(key, chunk)| (chunk.sub_chunks_len() != 0).then(|| key))
            .collect::<Vec<_>>();
        to_clear.into_iter().for_each(|key| {
            terrain.insert(key, Arc::clone(&empty));
        });
        drop(terrain);

        // Send a ping to the server once every second
        if self.state.get_program_time() - self.last_server_ping > 1. {
            self.send_msg_err(PingMsg::Ping)?;
            self.last_server_ping = self.state.get_program_time();
        }

        // 6) Update the server about the player's physics attributes.
        if self.presence.is_some() {
            if let (Some(pos), Some(vel), Some(ori)) = (
                self.state.read_storage().get(self.entity()).cloned(),
                self.state.read_storage().get(self.entity()).cloned(),
                self.state.read_storage().get(self.entity()).cloned(),
            ) {
                self.in_game_stream.send(ClientGeneral::PlayerPhysics {
                    pos,
                    vel,
                    ori,
                    force_counter: self.force_update_counter,
                })?;
            }
        }

        // 7) Finish the tick, pass control back to the frontend.
        self.tick += 1;

        Ok(())
    }

    /// another plugin data received, is this the last one
    pub fn plugin_received(&mut self, hash: PluginHash) -> usize {
        if !self.missing_plugins.remove(&hash) {
            tracing::warn!(?hash, "received unrequested plugin");
        }
        self.missing_plugins.len()
    }

    /// true if missing_plugins is not empty
    pub fn are_plugins_missing(&self) -> bool { !self.missing_plugins.is_empty() }

    /// extract list of locally cached plugins to load
    pub fn take_local_plugins(&mut self) -> Vec<PathBuf> { std::mem::take(&mut self.local_plugins) }
}

impl Drop for Client {
    fn drop(&mut self) {
        trace!("Dropping client");
        if self.registered {
            if let Err(e) = self.send_msg_err(ClientGeneral::Terminate) {
                warn!(
                    ?e,
                    "Error during drop of client, couldn't send disconnect package, is the \
                     connection already closed?",
                );
            }
        } else {
            trace!("no disconnect msg necessary as client wasn't registered")
        }

        tokio::task::block_in_place(|| {
            if let Err(e) = self
                .runtime
                .block_on(self.participant.take().unwrap().disconnect())
            {
                warn!(?e, "error when disconnecting, couldn't send all data");
            }
        });
        //explicitly drop the network here while the runtime is still existing
        drop(self.network.take());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use client_i18n::LocalizationHandle;

    #[test]
    /// THIS TEST VERIFIES THE CONSTANT API.
    /// CHANGING IT WILL BREAK 3rd PARTY APPLICATIONS (please extend) which
    /// needs to be informed (or fixed)
    ///  - torvus: https://gitlab.com/veloren/torvus
    ///
    /// CONTACT @Core Developer BEFORE MERGING CHANGES TO THIS TEST
    fn constant_api_test() {
        use common::clock::Clock;
        use voxygen_i18n_helpers::localize_chat_message;

        const SPT: f64 = 1.0 / 60.0;

        let runtime = Arc::new(Runtime::new().unwrap());
        let runtime2 = Arc::clone(&runtime);
        let username = "Foo";
        let password = "Bar";
        let auth_server = "auth.veloren.net";
        let veloren_client: Result<Client, Error> = runtime.block_on(Client::new(
            ConnectionArgs::Tcp {
                hostname: "127.0.0.1:9000".to_owned(),
                prefer_ipv6: false,
            },
            runtime2,
            &mut None,
            username,
            password,
            None,
            |suggestion: &str| suggestion == auth_server,
            &|_| {},
            |_| {},
            PathBuf::default(),
            ClientType::ChatOnly,
        ));
        let localisation = LocalizationHandle::load_expect("en");

        let _ = veloren_client.map(|mut client| {
            //clock
            let mut clock = Clock::new(Duration::from_secs_f64(SPT));

            //tick
            let events_result: Result<Vec<Event>, Error> =
                client.tick(ControllerInputs::default(), clock.dt());

            //chat functionality
            client.send_chat("foobar".to_string());

            let _ = events_result.map(|mut events| {
                // event handling
                if let Some(event) = events.pop() {
                    match event {
                        Event::Chat(msg) => {
                            let msg: comp::ChatMsg = msg;
                            let _s: String = localize_chat_message(
                                &msg,
                                &client.lookup_msg_context(&msg),
                                &localisation.read(),
                                true,
                            )
                            .1;
                        },
                        Event::Disconnect => {},
                        Event::DisconnectionNotification(_) => {
                            debug!("Will be disconnected soon! :/")
                        },
                        Event::Notification(notification) => {
                            let notification: UserNotification = notification;
                            debug!("Notification: {:?}", notification);
                        },
                        _ => {},
                    }
                };
            });

            client.cleanup();
            clock.tick();
        });
    }
}
