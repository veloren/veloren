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

async function sendGlobalMsg() {
    var world_msg = document.getElementById("world_msg");
    const msg_text = world_msg.value;

    const msg_response = await fetch("/ui_api/v1/send_world_msg", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
            msg: msg_text
        })
    });

    if (msg_response.status == 200) {
      world_msg.value = '';
    }
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