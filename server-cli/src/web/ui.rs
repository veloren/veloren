use axum::{
    extract::{ConnectInfo, State},
    http::{header::SET_COOKIE, HeaderValue},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::net::SocketAddr;

/// Keep Size small, so we dont have to Clone much for each request.
#[derive(Clone)]
struct UiApiToken {
    secret_token: String,
}

pub fn router(secret_token: String) -> Router {
    let token = UiApiToken { secret_token };
    Router::new().route("/", get(ui)).with_state(token)
}

async fn ui(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(token): State<UiApiToken>,
) -> impl IntoResponse {
    if !addr.ip().is_loopback() {
        return Html(
            r#"<!DOCTYPE html>
<html>
<body>
Ui is only accissable from 127.0.0.1
</body>
</html>
        "#
            .to_string(),
        )
        .into_response();
    }

    let mut response = Html(format!(
        r#"<!DOCTYPE html>
<html>
<head>
<script type="text/javascript">
{}
</script>
<style>
{}
</style>
</head>
<body>

{}

</body>
</html>"#,
        javascript(),
        css(),
        inner()
    ))
    .into_response();

    let cookie = format!("X-Secret-Token={}; SameSite=Strict", token.secret_token);

    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("An invalid secret-token for ui was provided"),
    );
    response
}

fn inner() -> &'static str {
    r##"
<div class="tab">
  <button class="tablinks active" onclick="openTab(event, 'settings')">Settings</button>
  <button class="tablinks" onclick="openTab(event, 'logs')">Logs</button>
  <button class="tablinks" onclick="openTab(event, 'players')">Players</button>
  <button class="tablinks" onclick="openTab(event, 'access')">Access</button>
</div>

<div id="settings" class="tabcontent">
  <div class="flex-container">
    <div class="first">
      <p>Server Name:</p>
    </div>
    <div>
      <p><input type="text" id="server_name" value="Veloren Alpha"></input></p>
    </div>
  </div>

  <div class="flex-container">
    <div class="first">
      <p>Server Ip/Port:</p>
    </div>
    <div>
      <p><input type="text" id="server_ip_port" value="0.0.0.0:14004"></input></p>
    </div>
  </div>

  <div class="flex-container">
    <div class="first">
      <p>Require Auth:</p>
    </div>
    <div>
      <p><input type="checkbox" checked="checked"></input> Enabled </p>
    </div>
  </div>

  <div class="flex-container">
    <div class="first">
      <p>Player Limit:</p>
    </div>
    <div>
      <p id="player_limit_no">20</p>
      <p><input type="range" min="1" max="100" value="20" class="slider" id="player_limit" oninput="changeSlider(event, 'player_limit', 'player_limit_no')"></p>
    </div>
  </div>

  <div class="flex-container">
    <div class="first">
      <p>Max View Distance:</p>
    </div>
    <div>
      <p id="view_distance_no">30</p>
      <p><input type="range" min="1" max="100" value="30" class="slider" id="view_distance" oninput="changeSlider(event, 'view_distance', 'view_distance_no')"></p>
    </div>
  </div>

  <div class="flex-container">
    <div class="first">
      <p>Global PvP:</p>
    </div>
    <div>
      <p><input type="checkbox"></input> Enable </p>
    </div>
  </div>

  <div class="flex-container">
    <div class="first">
      <p>Experimental Terrain Persistence:</p>
    </div>
    <div>
      <p><input type="checkbox"></input> Enable </p>
    </div>
  </div>
</div>

<div id="logs" class="tabcontent">
  <h3>Server Logs</h3>
  <div id="logs_list"></div>
</div>

<div id="players" class="tabcontent">
  <h3>Players</h3>
  <ul id="players_list">
  </ul>
</div>

<div id="access" class="tabcontent">
  <h3>Whitelist</h3>
  <h3>Banlist</h3>
  <h3>Admin</h3>
</div>
    "##
}

fn javascript() -> &'static str {
    r#"
function openTab(evt, cityName) {
  // Declare all variables
  var i, tabcontent, tablinks;

  // Get all elements with class="tabcontent" and hide them
  tabcontent = document.getElementsByClassName("tabcontent");
  for (i = 0; i < tabcontent.length; i++) {
    tabcontent[i].style.display = "none";
  }

  // Get all elements with class="tablinks" and remove the class "active"
  tablinks = document.getElementsByClassName("tablinks");
  for (i = 0; i < tablinks.length; i++) {
    tablinks[i].className = tablinks[i].className.replace(" active", "");
  }

  // Show the current tab, and add an "active" class to the button that opened the tab
  document.getElementById(cityName).style.display = "block";
  evt.currentTarget.className += " active";
}

function changeSlider(evt, sliderId, showId) {
    var slider = document.getElementById(sliderId);
    var sliderNo = document.getElementById(showId);
    sliderNo.innerHTML = slider.value;
}

async function update_players() {
    const players_response = await fetch("/ui_api/v1/players");
    const players = await players_response.json();

    // remove no longer existing childs
    var players_list = document.getElementById("players_list");
    for (var i = players_list.children.length-1; i >= 0; i--) {
      if (!players.includes(players_list.children[i].innerText)) {
        console.log("remove player: " + players_list.children[i].innerText);
        players_list.removeChild(players_list.children[i]);
        i--;
      }
    }

    // add non-existing elements
    addloop: for (const player of players) {
      for (var i = 0; i < players_list.children.length; i++) {
        if (players_list.children[i].innerText == player) {
          continue addloop;
        }
      }

      var li = document.createElement("li");
      li.appendChild(document.createTextNode(player));
      players_list.appendChild(li);

      console.log("added player: " + player);
    }
}

async function update_logs() {
    const logs_response = await fetch("/ui_api/v1/logs");
    const logs = await logs_response.json();

    // remove no longer existing childs
    var logs_list = document.getElementById("logs_list");
    while (logs_list.lastElementChild) {
      logs_list.removeChild(logs_list.lastElementChild);
    }

    for (const log of logs) {
      var p = document.createElement("p");
      p.appendChild(document.createTextNode(log));
      logs_list.appendChild(p);
    }
}

async function loop() {
    await update_players();
    await update_logs();
}

var loopId = window.setInterval(loop, 1000);
    "#
}

fn css() -> &'static str {
    r#"
 /* Style the tab */
.tab {
  overflow: hidden;
  border: 1px solid #ccc;
  background-color: #f1f1f1;
}

/* Style the buttons that are used to open the tab content */
.tab button {
  background-color: inherit;
  float: left;
  border: none;
  outline: none;
  cursor: pointer;
  padding: 14px 16px;
  transition: 0.3s;
}

/* Change background color of buttons on hover */
.tab button:hover {
  background-color: #ddd;
}

/* Create an active/current tablink class */
.tab button.active {
  background-color: #ccc;
}

/* Style the tab content */
.tabcontent {
  display: none;
  padding: 6px 12px;
  border: 1px solid #ccc;
  border-top: none;
}
div#settings.tabcontent  {
  display: block;
}

.flex-container {
  display: flex;
  margin: 4px;
  padding: 4px;
  background-color: #f1f1f1;
}

.flex-container .first {
    width: 300px
}
    "#
}
