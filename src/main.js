const { invoke } = window.__TAURI__.tauri;
const { emit, listen } = window.__TAURI__.event;
const { appWindow, WebviewWindow } = window.__TAURI__.window;
const { isPermissionGranted, requestPermission, sendNotification } =
  window.__TAURI__.notification;

let audio = new Audio("./assets/emergency-alarm.mp3");
let portListEl;
let payloadDataContainer;
let messageContainer;
let portStateContainer;
let esd_logo;
let logo_animation;
let port_status;
let alert_status;
let current_portname = "";
let permissionGranted = await isPermissionGranted();
if (!permissionGranted) {
  const permission = await requestPermission();
  permissionGranted = permission === "granted";
}

function saveSerialPortNumber(portNumber) {
  localStorage.setItem("serialPortNumber", portNumber);
}

function getSerialPortNumber() {
  return localStorage.getItem("serialPortNumber");
}

listen("alertData", (event) => {
  if (event.payload.data) {
    esd_logo.style.marginTop = "0px";
    alert_status = true;
    if (permissionGranted) {
      // sendNotification({
      //   title: "Emergency Shut Down (ESD)",
      //   body: `${event.payload.data}`,
      // });
    }
    setTimeout(() => {
      if (alert_status == true) {
        audio.play();
        logo_animation.play();
        payloadDataContainer.innerHTML = `${event.payload.data}`;
        document.getElementById("deactivateButton").style.display = "block";
        payloadDataContainer.style.backgroundColor = "red";
        messageContainer.innerHTML = `
    <p><img id="mtn-icon" src="./assets/mtn.svg" />Emergency Shut Down (ESD) has been initiated.</p><p><span style="color: white;"><u>Press Spacebar</u></span> to deactivate.</p>`;

        esd_logo.style.filter =
          "drop-shadow(0 0 2em #ffffff)invert(67%) sepia(89%) saturate(7492%) hue-rotate(346deg) brightness(84%) contrast(146%)";
      }
    }, 500);
    playAlarmSoundEffect();
  } else {
    logo_animation.restart();
    logo_animation.pause();
    audio.pause();
    audio.currentTime = 0;
    document.getElementById("deactivateButton").style.display = "none";
    alert_status = false;
    payloadDataContainer.innerHTML = "";
    payloadDataContainer.style.backgroundColor = "";
    esd_logo.style.filter =
      "drop-shadow(0 0 2em #ffffff) invert(42%) sepia(93%) saturate(1352%) hue-rotate(87deg) brightness(119%) contrast(119%)";
    messageContainer.innerHTML = `<span style="color: white;"><p>ESD has been successfully <span style="color: lightgreen;"><u>deactivated</u></span>.</p></span>`;
    setTimeout(() => {
      if (alert_status == false) {
        messageContainer.innerHTML = ""; //ESD deactivated
        esd_logo.style.marginTop = "100px";
      }
    }, 5000);
  }
});

listen("portState", (event) => {
  port_status = event.payload;

  if (port_status == "Connected") {
    portStateContainer.innerHTML = `Connected to Serial Port`;
    if (alert_status) return;
    esd_logo.style.filter =
      "drop-shadow(0 0 2em #ffffff) invert(42%) sepia(93%) saturate(1352%) hue-rotate(87deg) brightness(119%) contrast(119%)";
  } else if (port_status == "Port Busy") {
    portStateContainer.innerHTML = `Serial Port is Busy. Please try again later.`;
    if (alert_status) return;
    esd_logo.style.filter = "";
  } else {
    portStateContainer.innerHTML = `Disconnected`;
    if (alert_status) return;
    esd_logo.style.filter = "";
  }
});

async function listSerialPorts() {
  const ports = await invoke("list_serial_ports");
  portListEl.innerHTML = "";
  const defaultOption = document.createElement("option");
  defaultOption.text = current_portname;
  defaultOption.value = "";
  portListEl.add(defaultOption);
  defaultOption.style.display = "none";
  if (ports.length > 0) {
    for (const port of ports) {
      const option = document.createElement("option");
      option.value = port.port_name;
      option.text = port.port_name;
      option.style.backgroundColor = "#2A2F39";
      portListEl.add(option);
    }
  }
}

async function autoSelectPort() {
  const ports = await invoke("list_serial_ports");
  const savedPortNumber = getSerialPortNumber();
  portListEl.innerHTML = "";
  const defaultOption = document.createElement("option");
  defaultOption.text = current_portname;
  defaultOption.value = "";
  portListEl.add(defaultOption);
  defaultOption.style.display = "none";
  if (ports.length > 0) {
    for (const port of ports) {
      const option = document.createElement("option");
      option.value = port.port_name;
      option.text = port.port_name;
      option.style.backgroundColor = "#2A2F39";
      portListEl.add(option);
      if (savedPortNumber === port.port_name) {
        portListEl.value = savedPortNumber;
        startSerialCommunication();
      }
    }
  }
}

async function startSerialCommunication() {
  const selectedPort = portListEl.value;
  const baudRateElement = document.getElementById("baudrate-list");
  const selectedBaudRate = parseInt(baudRateElement.textContent, 10);
  while (port_status == "Connected") {
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  try {
    const response = await invoke("start_serial_communication", {
      portName: selectedPort,
      baudRate: selectedBaudRate,
    });
  } catch (error) {
    messageContainer.innerHTML = `<p>Error: ${error}</p>`;
  }
}

async function closeSerialPort() {
  const webview = new WebviewWindow("window");
  webview.emit("stopSerial");
}

async function sendCommandSerialPort() {
  const webview = new WebviewWindow("window");
  webview.emit("sendCommand");
}
window.addEventListener("DOMContentLoaded", () => {
  closeSerialPort();

  portListEl = document.querySelector("#port-list");
  alert_status = false;
  payloadDataContainer = document.querySelector("#payload-data");
  messageContainer = document.querySelector("#message-container");
  portStateContainer = document.querySelector("#port-state");
  esd_logo = document.querySelector("#esd-logo");

  logo_animation = anime({
    targets: esd_logo,
    scale: function (el, i, l) {
      return l - i * 2.5 - 0.3;
    },
    direction: "alternate",
    easing: "easeInOutSine",
    loop: true,
    duration: 300,
    autoplay: false,
  });

  portStateContainer.innerHTML = `Disconnected`;
  audio.loop = true;
  audio.pause();
  autoSelectPort();
  document.querySelector("#port-list").addEventListener("input", () => {
    closeSerialPort();
    portListEl.blur();
    const selectedPort = portListEl.value;
    current_portname = portListEl.value;
    if (selectedPort) {
      saveSerialPortNumber(selectedPort);
      startSerialCommunication();
    }
  });

  portListEl.addEventListener("focus", () => {
    listSerialPorts();
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === " " || event.code === "Space") {
      if (port_status == "Connected" && alert_status) {
        sendCommandSerialPort();
      }
    }
  });
  document.getElementById("deactivateButton").addEventListener("click", () => {
    if (port_status == "Connected" && alert_status) {
      sendCommandSerialPort();
    }
  });
});
