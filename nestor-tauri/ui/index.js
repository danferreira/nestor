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

const imageData = ctx.createImageData(WIDTH, HEIGHT);

async function emulate_next_frame() {
    const data = await invoke('next_frame');

    if (data && data.length > 0) {
        const display = new Uint8ClampedArray(data);
        imageData.data.set(display);

        const bitmap = await createImageBitmap(imageData);
        ctx.drawImage(bitmap, 0, 0, WIDTH, HEIGHT);
    }

    requestAnimationFrame(emulate_next_frame);
}

emulate_next_frame().catch(console.error)
