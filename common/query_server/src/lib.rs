use protocol::{
    wire::{self, dgram},
    Parcel,
};

#[cfg(feature = "client")] pub mod client;
pub mod proto;
#[cfg(feature = "server")] pub mod server;

fn create_pipeline<T: Parcel>() -> dgram::Pipeline<T, wire::middleware::pipeline::Default> {
    dgram::Pipeline::new(
        wire::middleware::pipeline::default(),
        protocol::Settings::default(),
    )
}
