const invoke = window.__TAURI__.invoke
const { emit, listen } = window.__TAURI__.event;


const WIDTH = 256
const HEIGHT = 240
const SCALE = 3
let anim_frame = 0

const canvas = document.getElementById("canvas")
canvas.width = WIDTH * SCALE
canvas.height = HEIGHT * SCALE

const ctx = canvas.getContext("2d")
ctx.fillStyle = "black"
ctx.fillRect(0, 0, WIDTH * SCALE, HEIGHT * SCALE)


console.log("Started")

async function run() {
    document.addEventListener("keydown", function (evt) {
        console.debug(evt)
        if (evt.key == "Escape") {
            if (anim_frame != 0) {
                window.cancelAnimationFrame(anim_frame)
            }
        }
        // invoke("keypress", { key: evt.key, pressed: true })
        //     .then((response) => console.log(response))
    })

    document.addEventListener("keyup", function (evt) {
        // invoke("keypress", { key: evt.key, pressed: false })
        //     .then((response) => console.log(response))
    })

    // var lastTimeStamp = new Date().getTime();

    listen("emulation_ready", async ev => {
        console.log("Start Emulation")
        invoke("start_emulation", "");
    })

    // listen("draw_frame", async ev => {
    //     const display = ev.payload

    //     let pixel = 0
    //     for (let i = 0; i < display.length; i += 3) {
    //         let color = `rgb(${display[i]} ${display[i + 1]} ${display[i + 2]})`;
    //         ctx.fillStyle = color;
    //         let x = pixel % WIDTH;
    //         let y = pixel / WIDTH;
    //         pixel++;
    //         ctx.fillRect(x * SCALE, y * SCALE, SCALE, SCALE)
    //     }

    //     frames++;
    // })


    // listen('draw_frame', ev => {
    //     const display = ev.payload; // This should be an array of pixel values
    //     for (let i = 0, j = 0; i < display.length; i += 3, j += 4) {
    //         imageData.data[j] = display[i];     // R
    //         imageData.data[j + 1] = display[i + 1]; // G
    //         imageData.data[j + 2] = display[i + 2]; // B
    //         imageData.data[j + 3] = 255;          // A
    //     }
    //     ctx.putImageData(imageData, 0, 0);
    //     frames++;
    // })

    listen('draw_frame', ev => {
        const display = new Uint8ClampedArray(ev.payload);

        // for (let i = 0, j = 0; i < display.length; i += 3, j += 4) {
        //     imageData.data[j] = display[i];     // R
        //     imageData.data[j + 1] = display[i + 1]; // G
        //     imageData.data[j + 2] = display[i + 2]; // B
        //     imageData.data[j + 3] = 255;          // A
        // }
        ctx.putImageData(new ImageData(display, WIDTH, HEIGHT), 0, 0);
    })

}

run().catch(console.error)
