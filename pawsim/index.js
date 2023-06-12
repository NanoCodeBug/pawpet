// Import our outputted wasm ES6 module
// Which, export default's, an initialization function
import init, { PawPetSim } from "./wasm_pawpet.js";

await init("./<VERSION>/wasm_pawpet_bg.wasm");


const pawpet = new PawPetSim();
let animationId = null;
let button_state = 0;
let paused = false;
const battery_slider = document.getElementById("battery");

const renderLoop = () => {
    pawpet.set_buttons(button_state);
    pawpet.set_battery(battery_slider.value);
    pawpet.tick();
};

const playPauseButton = document.getElementById("play_button");
const play = () => {
    console.log("play");
    paused = false;
    playPauseButton.textContent = "⏸️";
    animationId = setInterval(renderLoop, pawpet.get_framerate_ms() + 21);
    renderLoop();
};

const pause = () => {
    console.log("pause");
    playPauseButton.textContent = "▶️";
    clearInterval(animationId);
    paused = true
};

playPauseButton.addEventListener("click", event => {
    if (paused) {
        play();
    } else {
        pause();
    }
});

const FLAG_A = 0;
const FLAG_B = 1;
const FLAG_C = 2;
const FLAG_P = 3;
const FLAG_UP = 4;
const FLAG_RIGHT = 5;
const FLAG_DOWN = 6;
const FLAG_LEFT = 7;

document.addEventListener('keydown', (event) => {
    switch (event.code) {
        case "KeyW":
            button_state |= 1 << FLAG_UP;
            break;
        case "KeyD":
            button_state |= 1 << FLAG_RIGHT;
            break;
        case "KeyS":
            button_state |= 1 << FLAG_DOWN;
            break;
        case "KeyA":
            button_state |= 1 << FLAG_LEFT;
            break;
        case "KeyJ":
            button_state |= 1 << FLAG_A;
            break;
        case "KeyK":
            button_state |= 1 << FLAG_B;
            break;
        case "KeyL":
            button_state |= 1 << FLAG_C;
            break;
        case "Semicolon":
            button_state |= 1 << FLAG_P;
            break;
    }
});

document.addEventListener('keyup', (event) => {
    switch (event.code) {
        case "KeyW":
            button_state ^= 1 << FLAG_UP;
            break;
        case "KeyD":
            button_state ^= 1 << FLAG_RIGHT;
            break;
        case "KeyS":
            button_state ^= 1 << FLAG_DOWN;
            break;
        case "KeyA":
            button_state ^= 1 << FLAG_LEFT;
            break;
        case "KeyJ":
            button_state ^= 1 << FLAG_A;
            break;
        case "KeyK":
            button_state ^= 1 << FLAG_B;
            break;
        case "KeyL":
            button_state ^= 1 << FLAG_C;
            break;
        case "Semicolon":
            button_state ^= 1 << FLAG_P;
            break;
    }
});

let total_files_loaded = 0;

const LoadToFileSystem = (name, path) => {
    const req = new XMLHttpRequest();
    req.open("GET", "<VERSION>/"+path, true);
    req.responseType = "arraybuffer";
    req.onload = (event) => {
        const arrayBuffer = req.response; // Note: not req.responseText
        if (arrayBuffer) {
            const byteArray = new Uint8Array(arrayBuffer);

            console.log(path);
            console.log(name);

            pawpet.load_file(byteArray, name);

            total_files_loaded++;

            if (total_files == total_files_loaded) {
                play();
            }
        };
    };
    req.send(null);
};

let total_files = 4;
LoadToFileSystem("battery", "assets/battery.paw");
LoadToFileSystem("petsit", "assets/pet_sit.paw");
LoadToFileSystem("icons", "assets/icons.paw");
LoadToFileSystem("sleeptest", "assets/sleeptest.paw");
LoadToFileSystem("egg_wobble", "assets/egg_wobble.paw");
LoadToFileSystem("pet1_idle", "assets/pet1_idle.paw");
LoadToFileSystem("window", "assets/window.paw");

// TODO wait for assets to load before starting simulator

