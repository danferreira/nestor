const invoke = window.__TAURI__.invoke
const { emit, listen } = window.__TAURI__.event;

const WIDTH = 256
const HEIGHT = 240
const SCALE = 3

const canvas = document.getElementById("canvas")
canvas.width = WIDTH * SCALE;
canvas.height = HEIGHT * SCALE;

const ctx = canvas.getContext("2d")
ctx.scale(3, 3);
ctx.fillStyle = "black"
ctx.fillRect(0, 0, WIDTH, HEIGHT)
ctx.imageSmoothingEnabled = false;

var lastCalledTime;
var counter = 0;
var fpsArray = [];
const imageData = ctx.createImageData(WIDTH, HEIGHT);

async function emulate_next_frame(timestamp) {
    const data = await invoke('next_frame');

    if (data && data.length > 0) {
        const display = new Uint8ClampedArray(data);
        imageData.data.set(display);

        const bitmap = await createImageBitmap(imageData);
        ctx.drawImage(bitmap, 0, 0, WIDTH, HEIGHT);
    }

    var fps;

    if (!lastCalledTime) {
        lastCalledTime = new Date().getTime();
        fps = 0;
    }

    var delta = (new Date().getTime() - lastCalledTime) / 1000;
    lastCalledTime = new Date().getTime();
    fps = Math.ceil((1 / delta));

    if (counter >= 60) {
        var sum = fpsArray.reduce(function (a, b) { return a + b });
        var average = Math.ceil(sum / fpsArray.length);
        console.log(average);
        counter = 0;
    } else {
        if (fps !== Infinity) {
            fpsArray.push(fps);
        }

        counter++;
    }
    requestAnimationFrame(emulate_next_frame);
}

requestAnimationFrame(emulate_next_frame);
