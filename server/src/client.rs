use common::msg::ClientType;
use network::Participant;
use specs::Component;
use specs_idvs::IdvStorage;

/// Client handles ALL network related information of everything that connects
/// to the server Client DOES NOT handle game states
/// Client DOES NOT handle network information that is only relevant to some
/// "things" connecting to the server (there is currently no such case). First a
/// Client connects to the game, when it registers, it gets the `Player`
/// component, when he enters the game he gets the `InGame` component.
pub struct Client {
    pub client_type: ClientType,
    pub participant: Option<Participant>,
    pub last_ping: f64,
    pub login_msg_sent: bool,
    pub terminate_msg_recv: bool,
}

impl Component for Client {
    type Storage = IdvStorage<Self>;
}
