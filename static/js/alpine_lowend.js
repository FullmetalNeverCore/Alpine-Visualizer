import init, * as wasm from '/static/wasm/pkg/alpine_lowend_wasm.js'
const SCREEN_TEXT = 'SILENTWAVE F1 87.9'
const TARGET_FPS = 0
let RENDER_SCALE = 1
let FFT_SIZE = 256
const MEDIA_AUDIO = {
  echoCancellation:false,
  noiseSuppression:false,
  autoGainControl:false,
  channelCount:1
}

const FFT_SIZES = [64,128,256,512,1024,2048,4096,8192]

let analyser
let fftBuf
let ac
let currentStream
let hasAudioSource = false
let currentAudio = null
let canvas
let ready=false
let synthT=0
let imageData
let pixels
let ctx
let wasmMemory = null

function resizeCanvas(){
  if(!canvas) return
  const rect = canvas.getBoundingClientRect()
  const w = Math.max(1, Math.floor(rect.width))
  const h = Math.max(1, Math.floor(rect.height))
  const iw = Math.max(1, Math.floor(w * RENDER_SCALE))
  const ih = Math.max(1, Math.floor(h * RENDER_SCALE))
  if(canvas.width !== iw || canvas.height !== ih){
    canvas.width = iw
    canvas.height = ih
    if(wasmMemory){
      wasm.wasm_init_canvas(iw, ih)
      const ptr = wasm.frame_ptr()
      const len = wasm.frame_len()
      pixels = new Uint8ClampedArray(wasmMemory.buffer, ptr, len)
      if(ctx){ imageData = new ImageData(pixels, canvas.width, canvas.height) }
    }
  }
}

function setupTextCells(text = SCREEN_TEXT){
  const cells = document.getElementById('textCells')
  if(!cells) return
  cells.innerHTML = ''
  const screen = document.getElementById('screenText')
  if(screen) screen.innerHTML = ''
  for(let i=0;i<text.length;i++){
    const ch = text[i]
    const cell = document.createElement('div')
    cell.className = 'cell' + (ch===' ' ? ' space' : '')
    cell.style.animationDelay = (i * 0.1) + 's'
    cells.appendChild(cell)
    if(screen){
      const g = document.createElement('span')
      g.className = 'glyph'
      g.textContent = ch
      g.style.animationDelay = (i * 0.1) + 's'
      screen.appendChild(g)
    }
  }
}

function setupTrackInfoFlicker(){
  const trackInfo = document.getElementById('trackInfo')
  if(!trackInfo) return
  const items = trackInfo.querySelectorAll('.leftCol .item')
  items.forEach((el, idx)=>{ if(idx>0) el.classList.add('dim') })
}

async function refreshDevices(){
  const sel = document.getElementById('deviceSelect')
  if(!navigator.mediaDevices || !navigator.mediaDevices.enumerateDevices || !sel) return
  const devices = await navigator.mediaDevices.enumerateDevices()
  const inputs = devices.filter(d => d.kind === 'audioinput')
  sel.innerHTML = ''
  for(const d of inputs){
    const opt = document.createElement('option')
    opt.value = d.deviceId
    opt.textContent = d.label || 'Audio input'
    sel.appendChild(opt)
  }
}
async function setupFFT(){
  const sel = document.getElementById('fftSelect')
  if(!sel) return
  sel.innerHTML = ''
  for(const d of FFT_SIZES){
    const opt = document.createElement('option')
    opt.value = d
    opt.textContent = d
    sel.appendChild(opt)
  }
}

async function refreshFFT(){
  const sel = document.getElementById('fftSelect')
  if(!sel) return
  sel.value = FFT_SIZE
  sel.addEventListener('change', ()=>{
    FFT_SIZE = parseInt(sel.value)
    if(analyser){ analyser.fftSize = FFT_SIZE }
    if(fftBuf){ fftBuf = new Uint8Array(analyser.frequencyBinCount) }
  }, false)
}

function stopCurrentStream(){
  if(currentStream){
    currentStream.getTracks().forEach(t=>t.stop())
    currentStream = null
  }
  if(currentAudio){
    try{ currentAudio.pause() }catch(_){ }
    try{ URL.revokeObjectURL(currentAudio.src) }catch(_){ }
    currentAudio = null
  }
  hasAudioSource = false
}

function setupAnalyserFromStream(stream){
  stopCurrentStream()
  currentStream = stream
  if(!ac){ ac = new (window.AudioContext||window.webkitAudioContext)() }
  const src = ac.createMediaStreamSource(stream)
  analyser = ac.createAnalyser()
  analyser.smoothingTimeConstant = 0.34
  analyser.minDecibels = -90
  analyser.maxDecibels = -10
  analyser.fftSize = FFT_SIZE
  src.connect(analyser)
  fftBuf = new Uint8Array(analyser.frequencyBinCount)
  if(ac.state === 'suspended'){ ac.resume && ac.resume() }
  hasAudioSource = true
}

function setupFileAnalyser(file){
  stopCurrentStream()
  if(!ac){ ac = new (window.AudioContext||window.webkitAudioContext)() }
  const audio = new Audio()
  audio.src = URL.createObjectURL(file)
  audio.crossOrigin = 'anonymous'
  audio.loop = true
  audio.autoplay = true
  const src = ac.createMediaElementSource(audio)
  analyser = ac.createAnalyser()
  analyser.smoothingTimeConstant = 0.34
  analyser.minDecibels = -90
  analyser.maxDecibels = -10
  analyser.fftSize = FFT_SIZE
  src.connect(analyser)
  analyser.connect(ac.destination)
  fftBuf = new Uint8Array(analyser.frequencyBinCount)
  if(ac.state === 'suspended'){ ac.resume && ac.resume() }
  hasAudioSource = true
  currentAudio = audio
  const tryPlay = ()=>{ audio.play().catch(()=>{}) }
  audio.addEventListener('canplay', tryPlay, { once:true })
  setTimeout(tryPlay, 0)
}

async function start(){
  const inst = await init()
  wasmMemory = inst.memory
  canvas = document.getElementById('tunnelCanvas')
  window.addEventListener('resize', resizeCanvas, false)
  resizeCanvas()
  wasm.wasm_update_mouse(canvas.width/2, canvas.height/2, false, false)
  // expose wasm for console
  window.wasm = wasm
  window.setPerf = (n)=>{ try{ wasm.set_performance_mode && wasm.set_performance_mode(n) }catch(_){} }
  window.setRenderScale = (s)=>{ try{ RENDER_SCALE = Math.max(0.25, Math.min(1.0, Number(s)||1)); resizeCanvas() }catch(_){} }
  
  // Auto-detect low-end device by screen size and pixel ratio
  const isLowEnd = (window.innerWidth < 1290 || window.devicePixelRatio > 2) && 
                   navigator.hardwareConcurrency <= 2
  
  // Set performance mode based on device capabilities
  if(wasm.set_performance_mode) {
    // Default to mode 2 (performance) across devices; caller can override via window.setPerf
    wasm.set_performance_mode(2)
  }
  analyser = null
  fftBuf = new Uint8Array(4096)
  await refreshDevices()
  await setupFFT()
  await refreshFFT()
  const useBtn = document.getElementById('useDeviceBtn')
  const micBtn = document.getElementById('micBtn')
  const fileBtn = document.getElementById('loadFileBtn')
  const clearBtn = document.getElementById('clearFileBtn')
  const fileInput = document.getElementById('fileInput')
  const sel = document.getElementById('deviceSelect')
  if(useBtn && sel){
    useBtn.addEventListener('click', async ()=>{
      try{
        const deviceId = sel.value
        const constraints = deviceId ? { audio: { deviceId: { exact: deviceId }, ...MEDIA_AUDIO } } : { audio: MEDIA_AUDIO }
        const stream = await navigator.mediaDevices.getUserMedia(constraints)
        setupAnalyserFromStream(stream)
      }catch(_){ /* ignore */ }
    }, false)
  }
  if(micBtn){
    micBtn.style.display = 'inline-block'
    micBtn.addEventListener('click', async ()=>{
      try{
        const stream = await navigator.mediaDevices.getUserMedia({ audio: MEDIA_AUDIO })
        setupAnalyserFromStream(stream)
        await refreshDevices()
      }catch(_){ /* ignore */ }
    }, false)
  }
  // Typewriter effect function
  function typewriterEffect(element, text, speed = 100) {
    let i = 0;
    element.textContent = '';
    element.style.opacity = '1';

    function type() {
      if (i < text.length) {
        element.textContent += text.charAt(i);
        i++;
        setTimeout(type, speed);
      }
    }
    type();
  }

  if(fileBtn && fileInput){
    fileBtn.addEventListener('click', ()=> fileInput.click(), false)
    fileInput.addEventListener('change', ()=>{
      if(fileInput.files && fileInput.files[0]){
        setupFileAnalyser(fileInput.files[0])
        const name = fileInput.files[0].name || 'TRACK'
        const discId = document.querySelector('#trackInfo .discId')
        if(discId){ discId.textContent = 'D2' }
        const trckNum = document.querySelector('#trackInfo .trackNum')
        if(trckNum){ trckNum.textContent = '001' }
        const left = document.querySelectorAll('#trackInfo .leftCol .item')
        if(left.length>=2){ left[0].classList.add('dim'); left[1].classList.remove('dim'); left[1].classList.add('active'); left[0].classList.remove('active') }
        const titleLines = document.querySelectorAll('#trackInfo .title .line')
        if(titleLines[1]){ typewriterEffect(titleLines[1], name, 80) }
        if(clearBtn){ clearBtn.style.display = 'inline-block' }
      }
    }, false)
  }
  if(clearBtn){
    clearBtn.addEventListener('click', ()=>{
      stopCurrentStream()
      if(fileInput){ fileInput.value = '' }
      const discId = document.querySelector('#trackInfo .discId')
      if(discId){ discId.textContent = 'D1' }
      const left = document.querySelectorAll('#trackInfo .leftCol .item')
      if(left.length>=2){ left[1].classList.add('dim'); left[0].classList.remove('dim'); left[0].classList.add('active'); left[1].classList.remove('active') }
      const titleLines = document.querySelectorAll('#trackInfo .title .line')
      if(titleLines[1]){ typewriterEffect(titleLines[1], 'TRACK TITLE', 80) }
      if(clearBtn){ clearBtn.style.display = 'none' }
    }, false)
  }
  const ptr = wasm.frame_ptr()
  const len = wasm.frame_len()
  pixels = new Uint8ClampedArray(wasmMemory.buffer, ptr, len)
  ctx = canvas.getContext('2d')
  imageData = new ImageData(pixels, canvas.width, canvas.height)
  if(typeof wasm.wasm_set_screen_text === 'function'){
    wasm.wasm_set_screen_text(SCREEN_TEXT)
  } else {
    setupTextCells(SCREEN_TEXT)
  }
  setupTrackInfoFlicker()
  // fps label
  const fpsEl = document.getElementById('fpsCounter')
  window._alpineFps = { el: fpsEl, lastT: performance.now(), frames: 0, fps: 0 }
  ready=true
  loop()
}

let last = 0
const targetDt = TARGET_FPS > 0 ? 1000 / TARGET_FPS : 0
function loop(ts){
  if(ready){
    if(TARGET_FPS === 0 || ts - last >= targetDt){
      last = ts
      const active = hasAudioSource && analyser && ac && ac.state === 'running'
      if(active){
        analyser.getByteFrequencyData(fftBuf)
      } else if(fftBuf){
        fftBuf.fill(0)
      }
      wasm.wasm_update_vu(fftBuf)
      wasm.wasm_render_frame()
      if(pixels && ctx && imageData){ ctx.putImageData(imageData, 0, 0) }
      // update warptunnel fps once per 250ms
      const f = window._alpineFps
      if(f && f.el){
        f.frames++
        const dt = ts - f.lastT
        if(dt >= 250){ f.fps = Math.round((f.frames * 1000) / dt); f.frames = 0; f.lastT = ts; f.el.textContent = `WT ${f.fps} FPS` }
      }
    }
  }
  requestAnimationFrame(loop)
}

if(document.readyState==='complete' || document.readyState==='interactive'){
  setTimeout(start, 0)
} else {
  document.addEventListener('DOMContentLoaded', start)
}

