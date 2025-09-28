use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, Document, HtmlElement};

#[derive(Clone)]
struct Segment {
    x: f32,
    y: f32,
    x2d: f32,
    y2d: f32,
    index: i32,
    radius: f32,
    radius_audio: f32,
    segments: i32,
    audio_buffer_index: usize,
    active: bool,
    subs: Vec<Segment>,
}

struct CircleRow {
    segments_outside: Vec<Option<Segment>>,
    index: i32,
    z: f32,
    center_x: f32,
    center_y: f32,
    circle_center_x: f32,
    circle_center_y: f32,
    mp_x: f32,
    mp_y: f32,
    radius: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
}

static mut WIDTH: u32 = 0;
static mut HEIGHT: u32 = 0;
static mut VU: Option<Vec<u8>> = None;
static mut CIRCLES: Option<Vec<CircleRow>> = None;
static mut FRAME_BUFFER: Option<Vec<u8>> = None;
static mut TIME: f32 = 0.0;
static mut MOUSE_X: f32 = 0.0;
static mut MOUSE_Y: f32 = 0.0;
static mut MOUSE_ACTIVE: bool = false;
static mut MOUSE_DOWN: bool = false;
static mut COLOR_INVERT_VALUE: u8 = 0;
static mut VU_ACTIVE_BINS: usize = 0;
static mut VU_SEGS: Option<Vec<HtmlElement>> = None;
static mut VU_CUTS: Option<Vec<HtmlElement>> = None;
static mut VU_LOW: Option<Vec<HtmlElement>> = None;
static mut VU_MID: Option<Vec<HtmlElement>> = None;
static mut VU_HIGH: Option<Vec<HtmlElement>> = None;
static mut BEAT_ENV: f32 = 0.0;
static mut BEAT_BOOST: f32 = 0.0;
static mut BPM_EST: f32 = 0.0;
static mut VU_PREV_LOW: Option<Vec<f32>> = None;
static mut VU_PREV_MID: Option<Vec<f32>> = None;
static mut VU_PREV_HIGH: Option<Vec<f32>> = None;
static mut PREV_FRAME: Option<Vec<u8>> = None;
static mut TEMP_BUFFER: Option<Vec<u8>> = None;
static mut GLOW_BUFFER: Option<Vec<u8>> = None;
static mut MINI_VU_BLOCKS: Option<Vec<HtmlElement>> = None;

static mut RGB1_R: f32 = 0.0;
static mut RGB1_G: f32 = 0.0;
static mut RGB1_B: f32 = 0.0;
static mut RGB2_R: f32 = 0.0;
static mut RGB2_G: f32 = 0.0;
static mut RGB2_B: f32 = 0.0;

// Performance flags - can be configured
static mut LOW_END_MODE: bool = false;
static mut ENABLE_GLOW: bool = false;      // Default to perf mode (2): no glow
static mut ENABLE_SMOOTHING: bool = false; // Default to perf mode (2): no smoothing
static mut RENDER_SCALE: f32 = 1.0; // 0.5 for half resolution
static mut SKIP_FRAME_COUNT: u8 = 0;

const PI2: f32 = std::f32::consts::PI * 2.0;
const FOV: f32 = 400.0;
const SPEED: f32 = 0.6;
const FREQUENCY_DAMP: f32 = 18.0;
const PERSPECTIVE_OFFSET_X: f32 = -120.0;
const PERSPECTIVE_OFFSET_Y: f32 = -20.0;
const PERSPECTIVE_DEPTH_DIVISOR: f32 = 380.0;
const GLOW_BASE: f32 = 0.32;
const SMOOTHING: f32 = 0.45;  
const OUTER_GLOW_RADIUS: i32 = 4;     // balanced blur radius
const OUTER_GLOW_OPACITY: f32 = 0.70; // overall opacity of glow
const GLOW_GAMMA: f32 = 0.66;         // <1 brightens halo falloff
const BEAM_ADD: f32 = 0.85;           // additive beam strength
static mut COS_TABLE: Option<Vec<f32>> = None;
static mut SIN_TABLE: Option<Vec<f32>> = None;
const SATURATION_BOOST: f32 = 5.6;    // >1 increases color saturation
const FOG_NEAR_Z: f32 = FOV * 0.20;   // z where fog starts (far half of tunnel)
const FOG_FAR_Z: f32  = FOV * 0.92;   // z where fog is full
const FOG_BRIGHTNESS: f32 = 0.00;     // 0..1, 0 = black fog

#[wasm_bindgen]
pub fn set_low_end_mode(enabled: bool) {
    unsafe { 
        LOW_END_MODE = enabled;
        if enabled {
            ENABLE_GLOW = false;
            ENABLE_SMOOTHING = false;
        }
    }
}

#[wasm_bindgen]
pub fn set_render_scale(scale: f32) {
    unsafe { RENDER_SCALE = scale.clamp(0.25, 1.0); }
}

#[wasm_bindgen]
pub fn set_performance_mode(mode: u8) {
    unsafe {
        match mode {
            1 => { // Balanced - disable only heavy effects
                LOW_END_MODE = false;
                ENABLE_GLOW = false;
                ENABLE_SMOOTHING = true;
            }
            2 => { // Performance default - also enable low-end geometry
                LOW_END_MODE = false;
                ENABLE_GLOW = false;
                ENABLE_SMOOTHING = false;
            }
            _ => { // Maximum performance
                LOW_END_MODE = true;
                ENABLE_GLOW = false;
                ENABLE_SMOOTHING = false;
            }
        }
    }
}

#[wasm_bindgen]
pub fn wasm_init_canvas(width: u32, height: u32) {
    unsafe {
        WIDTH = width;
        HEIGHT = height;
        if VU.is_none() {
            VU = Some(vec![0u8; 8192]);
        }
        if COS_TABLE.is_none() || SIN_TABLE.is_none() {
            let segments = 64;
            let mut cos_t = Vec::with_capacity(segments as usize);
            let mut sin_t = Vec::with_capacity(segments as usize);
            for i in 0..segments {
                let ang = i as f32 * (PI2 / segments as f32);
                cos_t.push(ang.cos());
                sin_t.push(ang.sin());
            }
            COS_TABLE = Some(cos_t);
            SIN_TABLE = Some(sin_t);
        }
        RGB1_R = js_sys::Math::random() as f32 * PI2;
        RGB1_G = js_sys::Math::random() as f32 * PI2;
        RGB1_B = js_sys::Math::random() as f32 * PI2;
        RGB2_R = js_sys::Math::random() as f32 * PI2;
        RGB2_G = js_sys::Math::random() as f32 * PI2;
        RGB2_B = js_sys::Math::random() as f32 * PI2;
        if CIRCLES.is_none() {
            CIRCLES = Some(build_circles());
        }
        let needed = (width as usize) * (height as usize) * 4;
        match FRAME_BUFFER.as_mut() {
            Some(buf) if buf.len() == needed => {},
            Some(buf) => buf.resize(needed, 0),
            None => FRAME_BUFFER = Some(vec![0u8; needed]),
        }
        match PREV_FRAME.as_mut() {
            Some(buf) if buf.len() == needed => {},
            Some(buf) => buf.resize(needed, 0),
            None => PREV_FRAME = Some(vec![0u8; needed]),
        }
        match TEMP_BUFFER.as_mut() {
            Some(buf) if buf.len() == needed => {},
            Some(buf) => buf.resize(needed, 0),
            None => TEMP_BUFFER = Some(vec![0u8; needed]),
        }
        match GLOW_BUFFER.as_mut() {
            Some(buf) if buf.len() == needed => {},
            Some(buf) => buf.resize(needed, 0),
            None => GLOW_BUFFER = Some(vec![0u8; needed]),
        }
        if VU_SEGS.is_none() {
            if let Some(doc) = window().and_then(|w| w.document()) {
                if let Some(container) = doc.get_element_by_id("vuBars") {
                    container.set_inner_html("");
                    let mut segs: Vec<HtmlElement> = Vec::new();
                    let mut cuts: Vec<HtmlElement> = Vec::new();
                    let mut lows: Vec<HtmlElement> = Vec::new();
                    let mut mids: Vec<HtmlElement> = Vec::new();
                    let mut highs: Vec<HtmlElement> = Vec::new();
                    for _ in 0..24 {
                        let bar = doc.create_element("div").unwrap(); bar.set_attribute("class", "bar").ok();
                        let seg = doc.create_element("div").unwrap(); seg.set_attribute("class", "seg").ok();
                        let low = doc.create_element("div").unwrap(); low.set_attribute("class", "seg low").ok();
                        let mid = doc.create_element("div").unwrap(); mid.set_attribute("class", "seg mid").ok();
                        let high = doc.create_element("div").unwrap(); high.set_attribute("class", "seg high").ok();
                        let cut = doc.create_element("div").unwrap(); cut.set_attribute("class", "cut").ok();
                        seg.append_child(&low).ok(); seg.append_child(&mid).ok(); seg.append_child(&high).ok();
                        bar.append_child(&seg).ok(); bar.append_child(&cut).ok(); container.append_child(&bar).ok();
                        segs.push(seg.unchecked_into()); cuts.push(cut.unchecked_into());
                        lows.push(low.unchecked_into()); mids.push(mid.unchecked_into()); highs.push(high.unchecked_into());
                    }
                    VU_SEGS = Some(segs); VU_CUTS = Some(cuts); VU_LOW = Some(lows); VU_MID = Some(mids); VU_HIGH = Some(highs);
                    VU_PREV_LOW = Some(vec![0.0; 24]);
                    VU_PREV_MID = Some(vec![0.0; 24]);
                    VU_PREV_HIGH = Some(vec![0.0; 24]);
                }
            }
        }
    }
}

#[wasm_bindgen]
pub fn wasm_update_vu(buf: &[u8]) {
    unsafe {
        if let Some(v) = &mut VU {
            let copy_len = buf.len().min(v.len());
            v[..copy_len].copy_from_slice(&buf[..copy_len]);
            VU_ACTIVE_BINS = copy_len;
        }
    }
}

#[wasm_bindgen]
pub fn wasm_update_mouse(x: f32, y: f32, active: bool, down: bool) {
    unsafe {
        MOUSE_X = x;
        MOUSE_Y = y;
        MOUSE_ACTIVE = active;
        MOUSE_DOWN = down;
    }
}

#[wasm_bindgen]
pub fn frame_ptr() -> *const u8 {
    unsafe { FRAME_BUFFER.as_ref().unwrap().as_ptr() }
}

#[wasm_bindgen]
pub fn frame_len() -> usize {
    unsafe { FRAME_BUFFER.as_ref().unwrap().len() }
}

#[wasm_bindgen]
pub fn wasm_set_screen_text(s: &str) {
    let window = window().expect("no window");
    let document: Document = window.document().expect("no document");
    if let Some(screen) = document.get_element_by_id("screenText") {
        screen.set_inner_html("");
        for (i, ch) in s.chars().enumerate() {
            let span = document.create_element("span").unwrap();
            span.set_attribute("class", "glyph").ok();
            span.set_text_content(Some(&ch.to_string()));
            span.set_attribute("style", &format!("animation-delay: {}s", (i as f32) * 0.1)).ok();
            screen.append_child(&span).ok();
        }
    }
    if let Some(cells) = document.get_element_by_id("textCells") {
        cells.set_inner_html("");
        for (i, ch) in s.chars().enumerate() {
            let div = document.create_element("div").unwrap();
            let cls = if ch == ' ' { "cell space" } else { "cell" };
            div.set_attribute("class", cls).ok();
            div.set_attribute("style", &format!("animation-delay: {}s", (i as f32) * 0.1)).ok();
            cells.append_child(&div).ok();
        }
    }
    if document.get_element_by_id("lcdLabels").is_none() {
        if let Some(upper) = document.get_element_by_id("upperOverlay") {
            let labels = document.create_element("div").unwrap();
            labels.set_attribute("id", "lcdLabels").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("position", "absolute").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("left", "45%").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("top", "7.8vw").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("color", "#ffe07a").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("font-family", "DigitalDreamSkew, monospace").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("font-size", "1.1vw").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("letter-spacing", "0").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("text-shadow", "0 0 0.6vw rgba(255,224,122,.45)").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("z-index", "2").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("display", "flex").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("gap", "2.2vw").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("opacity", "0").ok();
            labels.dyn_ref::<HtmlElement>().unwrap().style().set_property("animation", "trackPowerOn 900ms ease-out forwards 150ms").ok();
            for (idx, word) in ["MIX", "RPT", "SCN"].iter().enumerate() {
                let span: HtmlElement = document.create_element("span").unwrap().unchecked_into();
                span.set_text_content(Some(word));
                span.style().set_property("opacity", "0").ok();
                span.style().set_property("transform", "translateY(0.2vw)").ok();
                let delay_ms = 180 + (idx as i32) * 80;
                span.style().set_property("animation", &format!("glowIn 600ms ease-out forwards {}ms", delay_ms)).ok();
                labels.append_child(&span).ok();
            }
            upper.append_child(&labels).ok();

            if document.get_element_by_id("miniVu").is_none() {
                let vu = document.create_element("div").unwrap();
                vu.set_attribute("id", "miniVu").ok();
                let style = vu.dyn_ref::<HtmlElement>().unwrap().style();
                style.set_property("position", "absolute").ok();
                style.set_property("left", "45%").ok();
                style.set_property("top", "6.2vw").ok();
                style.set_property("height", "0.6vw").ok();
                style.set_property("right", "5%").ok();
                style.set_property("display", "flex").ok();
                style.set_property("gap", "0.02vw").ok();
                style.set_property("align-items", "center").ok();
                style.set_property("opacity", "0").ok();
                style.set_property("animation", "trackPowerOn 700ms ease-out forwards 200ms").ok();

                let mut blocks: Vec<HtmlElement> = Vec::new();
                for _ in 0..19 {
                    let b: HtmlElement = document.create_element("div").unwrap().unchecked_into();
                    let bs = b.style();
                    bs.set_property("width", "0.4vw").ok();
                    bs.set_property("height", "100%").ok();
                    bs.set_property("background", "rgba(255,224,122,.25)").ok();
                    bs.set_property("box-shadow", "inset 0 0 2px rgba(0,0,0,.6)").ok();
                    bs.set_property("border", "1px solid rgba(255,255,255,.08)").ok();
                    vu.append_child(&b).ok();
                    blocks.push(b);
                }
                let min_label: HtmlElement = document.create_element("div").unwrap().unchecked_into();
                min_label.set_text_content(Some("MIN"));
                let min_style = min_label.style();
                min_style.set_property("position", "absolute").ok();
                min_style.set_property("left", "calc(52% - 3vw)").ok();
                min_style.set_property("top", "7.1vw").ok();
                min_style.set_property("color", "#ffe07a").ok();
                min_style.set_property("font-family", "DigitalDreamSkew, monospace").ok();
                min_style.set_property("font-size", "0.5vw").ok();
                min_style.set_property("text-shadow", "0 0 0.4vw rgba(255,224,122,.45)").ok();
                min_style.set_property("opacity", "0").ok();
                min_style.set_property("animation", "trackPowerOn 700ms ease-out forwards 200ms").ok();
                upper.append_child(&min_label).ok();

                let max_label: HtmlElement = document.create_element("div").unwrap().unchecked_into();
                max_label.set_text_content(Some("MAX"));
                let max_style = max_label.style();
                max_style.set_property("position", "absolute").ok();
                max_style.set_property("right", "calc(30% - 1vw)").ok();
                max_style.set_property("top", "7.1vw").ok();
                max_style.set_property("color", "#ffe07a").ok();
                max_style.set_property("font-family", "DigitalDreamSkew, monospace").ok();
                max_style.set_property("font-size", "0.5vw").ok();
                max_style.set_property("text-shadow", "0 0 0.4vw rgba(255,224,122,.45)").ok();
                max_style.set_property("opacity", "0").ok();
                max_style.set_property("animation", "trackPowerOn 700ms ease-out forwards 200ms").ok();
                upper.append_child(&max_label).ok();

                upper.append_child(&vu).ok();

                let bbe_label: HtmlElement = document.create_element("div").unwrap().unchecked_into();
                bbe_label.set_text_content(Some("B.B.E. OFF"));
                let bbe_style = bbe_label.style();
                bbe_style.set_property("position", "absolute").ok();
                bbe_style.set_property("left", "90%").ok();
                bbe_style.set_property("top", "6.8vw").ok();
                bbe_style.set_property("color", "#ffe07a").ok();
                bbe_style.set_property("font-family", "DigitalDreamSkew, monospace").ok();
                bbe_style.set_property("font-size", "0.6vw").ok();
                bbe_style.set_property("text-shadow", "0 0 0.4vw rgba(255,224,122,.45)").ok();
                bbe_style.set_property("opacity", "0").ok();
                bbe_style.set_property("animation", "trackPowerOn 700ms ease-out forwards 300ms").ok();
                upper.append_child(&bbe_label).ok();

                let preset_container: HtmlElement = document.create_element("div").unwrap().unchecked_into();
                let preset_style = preset_container.style();
                preset_style.set_property("position", "absolute").ok();
                preset_style.set_property("left", "90%").ok();
                preset_style.set_property("top", "7.5vw").ok();
                preset_style.set_property("display", "flex").ok();
                preset_style.set_property("gap", "0.3vw").ok();
                preset_style.set_property("opacity", "0").ok();
                preset_style.set_property("animation", "trackPowerOn 700ms ease-out forwards 400ms").ok();

                for (i, &num) in ["1", "2", "3", "4"].iter().enumerate() {
                    let btn: HtmlElement = document.create_element("div").unwrap().unchecked_into();
                    btn.set_text_content(Some(num));
                    let btn_style = btn.style();
                    btn_style.set_property("color", "#ffe07a").ok();
                    btn_style.set_property("font-family", "DigitalDreamSkew, monospace").ok();
                    btn_style.set_property("font-size", "0.6vw").ok();
                    btn_style.set_property("text-shadow", "0 0 0.4vw rgba(255,224,122,.45)").ok();
                    btn_style.set_property("background", "rgba(255,224,122,.1)").ok();
                    btn_style.set_property("border", "1px solid rgba(255,224,122,.3)").ok();
                    btn_style.set_property("border-radius", "2px").ok();
                    btn_style.set_property("padding", "0.2vw 0.4vw").ok();
                    btn_style.set_property("box-shadow", "inset 0 0 2px rgba(0,0,0,.6)").ok();
                    if i == 0 {
                        btn_style.set_property("background", "rgba(255,224,122,.3)").ok();
                        btn_style.set_property("border", "1px solid rgba(255,224,122,.6)").ok();
                    }
                    preset_container.append_child(&btn).ok();
                }
                let preset_label: HtmlElement = document.create_element("div").unwrap().unchecked_into();
                preset_label.set_text_content(Some("PRESET"));
                let preset_label_style = preset_label.style();
                preset_label_style.set_property("position", "absolute").ok();
                preset_label_style.set_property("left", "calc(90% - 2.5vw)").ok();
                preset_label_style.set_property("top", "7.9vw").ok();
                preset_label_style.set_property("color", "#ffe07a").ok();
                preset_label_style.set_property("font-family", "DigitalDreamSkew, monospace").ok();
                preset_label_style.set_property("font-size", "0.5vw").ok();
                preset_label_style.set_property("text-shadow", "0 0 0.4vw rgba(255,224,122,.45)").ok();
                preset_label_style.set_property("opacity", "0").ok();
                preset_label_style.set_property("animation", "trackPowerOn 700ms ease-out forwards 400ms").ok();
                upper.append_child(&preset_label).ok();

                upper.append_child(&preset_container).ok();
                unsafe { MINI_VU_BLOCKS = Some(blocks); }
            }
        }
    }
}

fn get_rgb_color1() -> (f32, f32, f32) {
    unsafe {
        RGB1_R += 0.040;
        RGB1_G += 0.028;
        RGB1_B += 0.052;
        let r = RGB1_R.sin() + 1.0;
        let g = RGB1_G.sin() + 1.0;
        let b = RGB1_B.sin() + 1.0;
        (r, g, b)
    }
}

fn get_rgb_color2() -> (f32, f32, f32) {
    unsafe {
        RGB2_R += 0.010;
        RGB2_G += 0.007;
        RGB2_B += 0.013;
        let r = RGB2_R.sin() + 1.0;
        let g = RGB2_G.sin() + 1.0;
        let b = RGB2_B.sin() + 1.0;
        (r, g, b)
    }
}

fn limit_color(r: f32, g: f32, b: f32, min_val: f32) -> (f32, f32, f32) {
    (r.max(min_val), g.max(min_val), b.max(min_val))
}

fn build_circles() -> Vec<CircleRow> {
    let mut rows = Vec::new();
    let mut index = 0;
    let audio_min = 8usize;
    let audio_max = 1024usize;
    let mp_x = js_sys::Math::random() as f32 * unsafe { WIDTH as f32 };
    let mp_y = js_sys::Math::random() as f32 * unsafe { HEIGHT as f32 };
    // Reduce circle count for low-end devices
    let step = if unsafe { LOW_END_MODE } { 10 } else { 5 };
    for z in (-FOV as i32..FOV as i32).step_by(step) {
        let radius = 75.0;
        let segments = if unsafe { LOW_END_MODE } { 48 } else { 64 };
        let mut segments_outside = Vec::new();
        let mut coords = Vec::new();
        for i in 0..=segments {
            let angle = (i as f32) * (PI2 / segments as f32) + unsafe { TIME };
            let x = angle.cos() * radius;
            let y = angle.sin() * radius;
            coords.push((x, y, i));
        }
        let toggle = index % 2;
        for i in 0..coords.len() {
            if (i as i32) % 2 == toggle {
                let audio_idx = audio_min + ((js_sys::Math::random() * ((audio_max - audio_min) as f64)) as usize);
                let (x, y, idx) = coords[i];
                let (prev_x, prev_y, prev_idx) = if i > 0 { 
                    coords[i - 1] 
                } else { 
                    coords[coords.len() - 2] 
                };
                let mut subs = Vec::new();
                subs.push(Segment {
                    x: prev_x, y: prev_y, x2d: 0.0, y2d: 0.0,
                    index: prev_idx, radius, radius_audio: radius,
                    segments, audio_buffer_index: audio_idx,
                    active: true, subs: vec![]
                });
                subs.push(Segment {
                    x, y, x2d: 0.0, y2d: 0.0,
                    index: idx, radius, radius_audio: radius,
                    segments, audio_buffer_index: audio_idx,
                    active: true, subs: vec![]
                });
                subs.push(Segment {
                    x: prev_x, y: prev_y, x2d: 0.0, y2d: 0.0,
                    index: prev_idx, radius, radius_audio: radius,
                    segments, audio_buffer_index: audio_idx,
                    active: true, subs: vec![]
                });
                subs.push(Segment {
                    x, y, x2d: 0.0, y2d: 0.0,
                    index: idx, radius, radius_audio: radius,
                    segments, audio_buffer_index: audio_idx,
                    active: true, subs: vec![]
                });
                subs.push(Segment {
                    x: prev_x, y: prev_y, x2d: 0.0, y2d: 0.0,
                    index: prev_idx, radius, radius_audio: radius,
                    segments, audio_buffer_index: audio_idx,
                    active: true, subs: vec![]
                });
                subs.push(Segment {
                    x, y, x2d: 0.0, y2d: 0.0,
                    index: idx, radius, radius_audio: radius,
                    segments, audio_buffer_index: audio_idx,
                    active: true, subs: vec![]
                });
                subs.push(Segment {
                    x: prev_x, y: prev_y, x2d: 0.0, y2d: 0.0,
                    index: prev_idx, radius, radius_audio: radius,
                    segments, audio_buffer_index: audio_idx,
                    active: true, subs: vec![]
                });
                let segment = Segment {
                    x, y, x2d: 0.0, y2d: 0.0,
                    index: idx, radius, radius_audio: radius,
                    segments, audio_buffer_index: audio_idx,
                    active: true, subs
                };
                segments_outside.push(Some(segment));
            } else {
                segments_outside.push(None);
            }
        }
        rows.push(CircleRow {
            segments_outside,
            index,
            z: z as f32,
            center_x: 0.0,
            center_y: 0.0,
            circle_center_x: 0.0,
            circle_center_y: 0.0,
            mp_x,
            mp_y,
            radius,
            color_r: 0.0,
            color_g: 0.0,
            color_b: 0.0,
        });
        index += 1;
    }
    rows
}

// Optimized line drawing with fewer boundary checks
#[inline(always)]
fn draw_line(buf: &mut [u8], w: usize, h: usize, x1: i32, y1: i32, x2: i32, y2: i32, r: u8, g: u8, b: u8) {
    let w_i32 = w as i32;
    let h_i32 = h as i32;
    
    // Early exit if line is completely outside
    if (x1 < 0 && x2 < 0) || (x1 >= w_i32 && x2 >= w_i32) ||
       (y1 < 0 && y2 < 0) || (y1 >= h_i32 && y2 >= h_i32) {
        return;
    }
    
    let mut x0 = x1;
    let mut y0 = y1;
    let dx = (x2 - x1).abs();
    let dy = (y2 - y1).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx - dy;
    
    let w4 = w * 4;
    
    loop {
        if x0 >= 0 && x0 < w_i32 && y0 >= 0 && y0 < h_i32 {
            let i = (y0 as usize) * w4 + (x0 as usize) * 4;
            unsafe {
                *buf.get_unchecked_mut(i) = r;
                *buf.get_unchecked_mut(i + 1) = g;
                *buf.get_unchecked_mut(i + 2) = b;
                *buf.get_unchecked_mut(i + 3) = 255;
            }
        }
        if x0 == x2 && y0 == y2 { break; }
        let e2 = 2 * err;
        if e2 > -dy { err -= dy; x0 += sx; }
        if e2 < dx { err += dx; y0 += sy; }
    }
}

fn soft_invert(buf: &mut [u8], value: u8) {
    for p in buf.chunks_exact_mut(4) {
        p[0] = (value as i32 - p[0] as i32).abs() as u8;
        p[1] = (value as i32 - p[1] as i32).abs() as u8;
        p[2] = (value as i32 - p[2] as i32).abs() as u8;
        p[3] = 255;
    }
}

fn draw_thick_line(buf: &mut [u8], w: usize, h: usize, x1: i32, y1: i32, x2: i32, y2: i32, r: u8, g: u8, b: u8, t: i32) {
    let half = t.max(1) / 2;
    if x1 == x2 {
        for o in -half..=half { draw_line(buf, w, h, x1 + o, y1, x2 + o, y2, r, g, b); }
    } else if y1 == y2 {
        for o in -half..=half { draw_line(buf, w, h, x1, y1 + o, x2, y2 + o, r, g, b); }
    } else {
        for o in -half..=half { draw_line(buf, w, h, x1 + o, y1, x2 + o, y2, r, g, b); }
        for o in -half..=half { draw_line(buf, w, h, x1, y1 + o, x2, y2 + o, r, g, b); }
    }
}

fn render_lcd_char(frame: &mut [u8], w: usize, h: usize, ch: char, x: i32, y: i32, cw: i32, chh: i32, color: (u8, u8, u8)) {
    let t = 2;
    match ch {
        'M' => {
            draw_thick_line(frame, w, h, x, y, x, y + chh, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x + cw, y, x + cw, y + chh, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x, y, x + cw / 2, y + chh / 2, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x + cw / 2, y + chh / 2, x + cw, y, color.0, color.1, color.2, t);
        }
        'I' => {
            draw_thick_line(frame, w, h, x, y, x + cw, y, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x + cw / 2, y, x + cw / 2, y + chh, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x, y + chh, x + cw, y + chh, color.0, color.1, color.2, t);
        }
        'X' => {
            draw_thick_line(frame, w, h, x, y, x + cw, y + chh, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x + cw, y, x, y + chh, color.0, color.1, color.2, t);
        }
        'R' => {
            draw_thick_line(frame, w, h, x, y, x, y + chh, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x, y, x + cw, y, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x + cw / 2, y + chh / 2, x + cw, y + chh / 2, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x + cw, y, x + cw, y + chh / 2, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x + cw / 2, y + chh / 2, x + cw, y + chh, color.0, color.1, color.2, t);
        }
        'P' => {
            draw_thick_line(frame, w, h, x, y, x, y + chh, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x, y, x + cw, y, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x, y + chh / 2, x + cw, y + chh / 2, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x + cw, y, x + cw, y + chh / 2, color.0, color.1, color.2, t);
        }
        'T' => {
            draw_thick_line(frame, w, h, x, y, x + cw, y, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x + cw / 2, y, x + cw / 2, y + chh, color.0, color.1, color.2, t);
        }
        'S' => {
            draw_thick_line(frame, w, h, x, y, x + cw, y, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x + cw, y, x + cw, y + chh / 2, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x, y + chh / 2, x + cw, y + chh / 2, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x, y + chh / 2, x, y + chh, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x, y + chh, x + cw, y + chh, color.0, color.1, color.2, t);
        }
        'C' => {
            draw_thick_line(frame, w, h, x, y, x + cw, y, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x, y + chh, x + cw, y + chh, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x, y, x, y + chh, color.0, color.1, color.2, t);
        }
        'N' => {
            draw_thick_line(frame, w, h, x, y, x, y + chh, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x + cw, y, x + cw, y + chh, color.0, color.1, color.2, t);
            draw_thick_line(frame, w, h, x, y, x + cw, y + chh, color.0, color.1, color.2, t);
        }
        _ => {}
    }
}

fn render_lcd_string(frame: &mut [u8], w: usize, h: usize, text: &str, x: i32, y: i32, cw: i32, chh: i32, color: (u8, u8, u8)) {
    for (i, ch) in text.chars().enumerate() {
        let cx = x + (i as i32) * (cw + 4);
        render_lcd_char(frame, w, h, ch, cx, y, cw, chh, color);
    }
}


#[wasm_bindgen]
pub fn wasm_render_frame() {
    let (w, h) = unsafe { (WIDTH as usize, HEIGHT as usize) };
    if w == 0 || h == 0 { return; }
    
    // Skip expensive effects on low-end mode
    let enable_glow = unsafe { ENABLE_GLOW && !LOW_END_MODE };
    let enable_smoothing = unsafe { ENABLE_SMOOTHING && !LOW_END_MODE };

    let frame = unsafe {
        let needed = w * h * 4;
        match FRAME_BUFFER.as_mut() {
            Some(buf) if buf.len() == needed => buf,
            Some(buf) => { buf.resize(needed, 0); buf },
            None => { FRAME_BUFFER = Some(vec![0u8; needed]); FRAME_BUFFER.as_mut().unwrap() }
        }
    };
    
    // Optimized clear - use ptr::write_bytes for better performance
    unsafe {
        std::ptr::write_bytes(frame.as_mut_ptr(), 0, frame.len());
        // Set alpha channel
        for i in (3..frame.len()).step_by(4) {
            *frame.get_unchecked_mut(i) = 255;
        }
    }
    
    let vu_data = unsafe { VU.as_ref().unwrap() };
    // Cache this to avoid repeated checks
    let _has_audio = unsafe { VU_ACTIVE_BINS > 0 };
    let (energy_avg, bass_avg, beat_boost) = unsafe {
        let bins = VU_ACTIVE_BINS.max(1);
        let mut sum: u32 = 0; for v in vu_data.iter().take(bins) { sum += *v as u32; }
        let ea = (sum as f32) / (bins as f32) / 255.0;
        let bbins = if bins > 4 { 4 } else { bins };
        let mut s = 0u32; for v in vu_data.iter().take(bbins){ s += *v as u32; }
        let ba = (s as f32) / (bbins as f32) / 255.0;
        let env_attack = 0.90f32;
        let env_release = 0.98f32;
        let prev = BEAT_ENV;
        let env = if ba > prev { prev * env_attack + ba * (1.0 - env_attack) } else { prev * env_release + ba * (1.0 - env_release) };
        BEAT_ENV = env;
        let pulse = (ba - env - 0.02).max(0.0);
        BEAT_BOOST = BEAT_BOOST * 0.90 + pulse * 28.0;
        (ea, ba, BEAT_BOOST.min(6.5))
    };
    
    let (col_r, col_g, col_b) = get_rgb_color1();
    let (col2_r, col2_g, col2_b) = limit_color(*&get_rgb_color2().0, *&get_rgb_color2().1, *&get_rgb_color2().2, 0.25);
    
    let mut sort_needed = false;
    
    unsafe {
        if let Some(circles) = CIRCLES.as_mut() {
            let total = circles.len();
            
            for i in 0..total {
                let (head, tail) = circles.split_at_mut(i);
                let circle = &mut tail[0];
                let prev_opt = if i > 0 { Some(&head[i - 1]) } else { None };
                
                // Update colors
                circle.color_r = col_r - (circle.z + FOV) / FOV;
                circle.color_g = col_g - (circle.z + FOV) / FOV;
                circle.color_b = col_b - (circle.z + FOV) / FOV;
                
                circle.color_r = circle.color_r.max(col2_r);
                circle.color_g = circle.color_g.max(col2_g);
                circle.color_b = circle.color_b.max(col2_b);
                
                circle.mp_x = (w as f32 / 2.0) + PERSPECTIVE_OFFSET_X;
                circle.mp_y = (h as f32 / 2.0) + PERSPECTIVE_OFFSET_Y;
                
                // Calculate center with perspective
                circle.center_x = ((w as f32 / 2.0) - circle.mp_x) * ((circle.z - FOV) / PERSPECTIVE_DEPTH_DIVISOR) + w as f32 / 2.0;
                circle.center_y = ((h as f32 / 2.0) - circle.mp_y) * ((circle.z - FOV) / PERSPECTIVE_DEPTH_DIVISOR) + h as f32 / 2.0;
                
                let scale = FOV / (FOV + circle.z);
                let scale_back = prev_opt.map(|p| FOV / (FOV + p.z)).unwrap_or(scale);
                
                // Frustum culling: skip rows fully outside viewport
                let min_x = (circle.center_x - circle.radius).floor() as i32;
                let max_x = (circle.center_x + circle.radius).ceil() as i32;
                let min_y = (circle.center_y - circle.radius).floor() as i32;
                let max_y = (circle.center_y + circle.radius).ceil() as i32;
                let w_i32 = w as i32;
                let h_i32 = h as i32;
                let offscreen = max_x < 0 || min_x >= w_i32 || max_y < 0 || min_y >= h_i32;
                if offscreen { continue; }

                // Process segments
                for j in 0..circle.segments_outside.len() {
                    if let Some(seg) = circle.segments_outside[j].as_mut() {
                        seg.x2d = seg.x * scale + circle.center_x;
                        seg.y2d = seg.y * scale + circle.center_y;
                        
                        let frequency = vu_data[seg.audio_buffer_index % vu_data.len()] as f32;
                        let frequency_add = frequency / FREQUENCY_DAMP * (1.0 + beat_boost*0.3);
                        
                        seg.radius_audio = seg.radius - frequency_add;
                        
                        // Skip complex geometry on distant rows for performance
                        if unsafe { LOW_END_MODE } && circle.z.abs() > FOV * 0.8 {
                            continue;
                        }
                        
                        // Draw 3D faces
                        if i > 0 && i < total - 1 && seg.subs.len() >= 7 {
                            let brightness_base = 20.0 + energy_avg * 200.0;
                            let line_value = ((i as f32 / total as f32) * (brightness_base + frequency)).min(255.0);

                            // Fog only for far side (positive z). Near (negative z) remains clear.
                            let mut fog = if circle.z <= FOG_NEAR_Z { 0.0 } else { ((circle.z - FOG_NEAR_Z) / (FOG_FAR_Z - FOG_NEAR_Z)).clamp(0.0, 1.0) };
                            // smoothstep
                            fog = fog * fog * (3.0 - 2.0 * fog);
                            if fog >= 0.999 { continue; }
                            
                            // Increase saturation by pushing away from gray
                            let mut crf = circle.color_r * line_value;
                            let mut cgf = circle.color_g * line_value;
                            let mut cbf = circle.color_b * line_value;
                            let luma = (crf + cgf + cbf) / 3.0;
                            crf = (luma + (crf - luma) * SATURATION_BOOST).min(255.0);
                            cgf = (luma + (cgf - luma) * SATURATION_BOOST).min(255.0);
                            cbf = (luma + (cbf - luma) * SATURATION_BOOST).min(255.0);
                            // Apply fog (mix toward fog brightness)
                            let fog_mix = fog;
                            crf = crf * (1.0 - fog_mix) + (FOG_BRIGHTNESS * 255.0) * fog_mix;
                            cgf = cgf * (1.0 - fog_mix) + (FOG_BRIGHTNESS * 255.0) * fog_mix;
                            cbf = cbf * (1.0 - fog_mix) + (FOG_BRIGHTNESS * 255.0) * fog_mix;

                            let white_mix = ((beat_boost / 6.5).min(5.0)).powf(1.8) * 0.6;
                            if white_mix > 0.0 {
                                crf = crf + (255.0 - crf) * white_mix;
                                cgf = cgf + (255.0 - cgf) * white_mix;
                                cbf = cbf + (255.0 - cbf) * white_mix;
                            }
                            let cr = crf as u8;
                            let cg = cgf as u8;
                            let cb = cbf as u8;
                            
                            // Use lookup tables for trig functions
                            let cos_sin_table = unsafe { (COS_TABLE.as_ref().unwrap(), SIN_TABLE.as_ref().unwrap()) };
                            let seg_count = seg.segments as usize;
                            
                            // Helper to get trig values from lookup table
                            let get_trig = |index: i32| -> (f32, f32) {
                                let idx = (index as usize) % seg_count;
                                (cos_sin_table.0[idx], cos_sin_table.1[idx])
                            };
                            
                            // sub1 (index 0) - current row, audio radius
                            let (cos_val, sin_val) = get_trig(seg.subs[0].index);
                            seg.subs[0].x = circle.circle_center_x + cos_val * seg.radius_audio;
                            seg.subs[0].y = circle.circle_center_y + sin_val * seg.radius_audio;
                            seg.subs[0].x2d = seg.subs[0].x * scale + circle.center_x;
                            seg.subs[0].y2d = seg.subs[0].y * scale + circle.center_y;
                            
                            // sub2 (index 1) - back row, audio radius
                            let (cos_val, sin_val) = get_trig(seg.subs[1].index);
                            seg.subs[1].x = circle.circle_center_x + cos_val * seg.radius_audio;
                            seg.subs[1].y = circle.circle_center_y + sin_val * seg.radius_audio;
                            if let Some(prev) = prev_opt {
                                seg.subs[1].x2d = seg.subs[1].x * scale_back + prev.center_x;
                                seg.subs[1].y2d = seg.subs[1].y * scale_back + prev.center_y;
                            }
                            
                            // sub3 (index 2) - back row, audio radius
                            let (cos_val, sin_val) = get_trig(seg.subs[2].index);
                            seg.subs[2].x = circle.circle_center_x + cos_val * seg.radius_audio;
                            seg.subs[2].y = circle.circle_center_y + sin_val * seg.radius_audio;
                            if let Some(prev) = prev_opt {
                                seg.subs[2].x2d = seg.subs[2].x * scale_back + prev.center_x;
                                seg.subs[2].y2d = seg.subs[2].y * scale_back + prev.center_y;
                            }
                            
                            // sub4 (index 3) - current row, normal radius
                            let (cos_val, sin_val) = get_trig(seg.subs[3].index);
                            seg.subs[3].x = circle.circle_center_x + cos_val * seg.radius;
                            seg.subs[3].y = circle.circle_center_y + sin_val * seg.radius;
                            seg.subs[3].x2d = seg.subs[3].x * scale + circle.center_x;
                            seg.subs[3].y2d = seg.subs[3].y * scale + circle.center_y;
                            
                            // sub5 (index 4) - current row, normal radius
                            let (cos_val, sin_val) = get_trig(seg.subs[4].index);
                            seg.subs[4].x = circle.circle_center_x + cos_val * seg.radius;
                            seg.subs[4].y = circle.circle_center_y + sin_val * seg.radius;
                            seg.subs[4].x2d = seg.subs[4].x * scale + circle.center_x;
                            seg.subs[4].y2d = seg.subs[4].y * scale + circle.center_y;
                            
                            // sub6 (index 5) - back row, normal radius
                            let (cos_val, sin_val) = get_trig(seg.subs[5].index);
                            seg.subs[5].x = circle.circle_center_x + cos_val * seg.radius;
                            seg.subs[5].y = circle.circle_center_y + sin_val * seg.radius;
                            if let Some(prev) = prev_opt {
                                seg.subs[5].x2d = seg.subs[5].x * scale_back + prev.center_x;
                                seg.subs[5].y2d = seg.subs[5].y * scale_back + prev.center_y;
                            }
                            
                            // sub7 (index 6) - back row, normal radius  
                            let (cos_val, sin_val) = get_trig(seg.subs[6].index);
                            seg.subs[6].x = circle.circle_center_x + cos_val * seg.radius;
                            seg.subs[6].y = circle.circle_center_y + sin_val * seg.radius;
                            if let Some(prev) = prev_opt {
                                seg.subs[6].x2d = seg.subs[6].x * scale_back + prev.center_x;
                                seg.subs[6].y2d = seg.subs[6].y * scale_back + prev.center_y;
                            }
                            
                            // Draw faces
                            let p1 = (seg.x2d as i32, seg.y2d as i32);
                            let p2 = (seg.subs[1].x2d as i32, seg.subs[1].y2d as i32);
                            let p3 = (seg.subs[2].x2d as i32, seg.subs[2].y2d as i32);
                            let p4 = (seg.subs[0].x2d as i32, seg.subs[0].y2d as i32);
                            let p5 = (seg.subs[3].x2d as i32, seg.subs[3].y2d as i32);
                            let p6 = (seg.subs[4].x2d as i32, seg.subs[4].y2d as i32);
                            let p7 = (seg.subs[6].x2d as i32, seg.subs[6].y2d as i32);
                            let p8 = (seg.subs[5].x2d as i32, seg.subs[5].y2d as i32);
                            
                            // Draw inner face only if there's audio deformation
                            if frequency_add > 0.0 {
                                draw_line(frame, w, h, p1.0, p1.1, p2.0, p2.1, cr, cg, cb);
                                draw_line(frame, w, h, p2.0, p2.1, p3.0, p3.1, cr, cg, cb);
                                draw_line(frame, w, h, p3.0, p3.1, p4.0, p4.1, cr, cg, cb);
                                draw_line(frame, w, h, p4.0, p4.1, p1.0, p1.1, cr, cg, cb);
                                
                                // Connecting edges
                                draw_line(frame, w, h, p5.0, p5.1, p1.0, p1.1, cr, cg, cb);
                                draw_line(frame, w, h, p6.0, p6.1, p4.0, p4.1, cr, cg, cb);
                                draw_line(frame, w, h, p7.0, p7.1, p3.0, p3.1, cr, cg, cb);
                                draw_line(frame, w, h, p8.0, p8.1, p2.0, p2.1, cr, cg, cb);
                            }
                            
                            // Outer face (if close enough)
                            if circle.z < FOV / 3.0 && !unsafe { LOW_END_MODE } {
                                draw_line(frame, w, h, p5.0, p5.1, p6.0, p6.1, cr, cg, cb);
                                draw_line(frame, w, h, p6.0, p6.1, p7.0, p7.1, cr, cg, cb);
                                draw_line(frame, w, h, p7.0, p7.1, p8.0, p8.1, cr, cg, cb);
                                draw_line(frame, w, h, p8.0, p8.1, p5.0, p5.1, cr, cg, cb);
                            }
                        }
                        
                        // Update segment position using lookup tables
                        let cos_sin_table = unsafe { (COS_TABLE.as_ref().unwrap(), SIN_TABLE.as_ref().unwrap()) };
                        let idx = (seg.index as usize) % (seg.segments as usize);
                        seg.x = circle.circle_center_x + cos_sin_table.0[idx] * seg.radius_audio;
                        seg.y = circle.circle_center_y + cos_sin_table.1[idx] * seg.radius_audio;
                    }
                }
                
                // Update Z position with beat-reactive speed
                let bpm_wave = (beat_boost * 0.3).min(2.0);
                let dynamic_speed = SPEED + beat_boost + bpm_wave;
                if MOUSE_DOWN {
                    circle.z += dynamic_speed;
                    if circle.z > FOV {
                        circle.z -= FOV * 2.0;
                        sort_needed = true;
                    }
                } else {
                    circle.z -= dynamic_speed;
                    if circle.z < -FOV {
                        circle.z += FOV * 2.0;
                        sort_needed = true;
                    }
                }
            }
            
            // Sort by depth if needed
            if sort_needed {
                circles.sort_by(|a, b| b.z.partial_cmp(&a.z).unwrap());
            }
        }
        
        // Update time
        if MOUSE_DOWN {
            TIME -= 0.005;
        } else {
            TIME += 0.005;
        }
        
        if let (Some(segs), Some(cuts), Some(lows), Some(mids), Some(highs)) = (VU_SEGS.as_ref(), VU_CUTS.as_ref(), VU_LOW.as_ref(), VU_MID.as_ref(), VU_HIGH.as_ref()) {
            let cols = segs.len();
            let bins = VU_ACTIVE_BINS.max(1);
            let bands_per_col = 3usize;
            let buckets_total = cols * bands_per_col;
            let bucket_size = ((bins + buckets_total - 1) / buckets_total).max(1);
            let quant = |pct: f32| -> f32 { let step = 6.0f32; (pct / step).floor() * step };
            for i in 0..cols {
                let base = (i * bands_per_col * bucket_size).min(bins - 1);
                let mut acc_low = 0u32; let mut acc_mid = 0u32; let mut acc_high = 0u32;
                for k in 0..bucket_size { let idx = base + k; if idx < bins { acc_low += vu_data[idx] as u32; } }
                for k in 0..bucket_size { let idx = base + bucket_size + k; if idx < bins { acc_mid += vu_data[idx] as u32; } }
                for k in 0..bucket_size { let idx = base + bucket_size*2 + k; if idx < bins { acc_high += vu_data[idx] as u32; } }
                let mut v_low = (acc_low as f32 / bucket_size as f32) / 255.0;
                let mut v_mid = (acc_mid as f32 / bucket_size as f32) / 255.0;
                let mut v_high = (acc_high as f32 / bucket_size as f32) / 255.0;
                let noise = 0.12f32;
                v_low = (v_low - noise).max(0.0) * 0.9;
                v_mid = (v_mid - noise).max(0.0) * 0.8;
                v_high = (v_high - noise).max(0.0) * 0.7;
                let mut h_low = quant(v_low * 100.0).max(2.0);
                let mut h_mid = quant(v_mid * 100.0).max(0.0);
                let mut h_high = quant(v_high * 100.0).max(0.0);
                if let (Some(prev_l), Some(prev_m), Some(prev_h)) = (VU_PREV_LOW.as_mut(), VU_PREV_MID.as_mut(), VU_PREV_HIGH.as_mut()) {
                    let rl = prev_l[i]; let rm = prev_m[i]; let rh = prev_h[i];
                    let release = 0.85f32;
                    h_low = h_low.max(rl * release);
                    h_mid = h_mid.max(rm * release);
                    h_high = h_high.max(rh * release);
                    prev_l[i] = h_low; prev_m[i] = h_mid; prev_h[i] = h_high;
                }
                let mut total_h = h_low + h_mid + h_high;
                if total_h > 100.0 {
                    let scale = 100.0 / total_h;
                    h_low = quant(h_low * scale);
                    h_mid = quant(h_mid * scale);
                    h_high = quant(h_high * scale);
                    total_h = (h_low + h_mid + h_high).min(100.0);
                }
                segs[i].style().set_property("height", "100%").ok();
                cuts[i].style().set_property("bottom", &format!("{}%", total_h.min(100.0))).ok();
                lows[i].style().set_property("height", &format!("{}%", h_low)).ok();
                mids[i].style().set_property("height", &format!("{}%", h_mid)).ok();
                highs[i].style().set_property("height", &format!("{}%", h_high)).ok();
                lows[i].style().set_property("bottom", "0%" ).ok();
                let mid_bottom = h_low.min(100.0);
                mids[i].style().set_property("bottom", &format!("{}%", mid_bottom)).ok();
                let high_bottom = (h_low + h_mid).min(100.0);
                highs[i].style().set_property("bottom", &format!("{}%", high_bottom)).ok();
            }
        }
        } 

        if let Some(blocks) = unsafe { MINI_VU_BLOCKS.as_ref() } {
            let bins = unsafe { VU_ACTIVE_BINS.max(1) };
            let vu_data = unsafe { VU.as_ref().unwrap() };
            let cols = blocks.len();
            let bucket = ((bins + cols - 1) / cols).max(1);
            for i in 0..cols {
                let mut acc = 0u32;
                for k in 0..bucket { let idx = i * bucket + k; if idx < bins { acc += vu_data[idx] as u32; } }
                let v = (acc as f32 / bucket as f32) / 255.0;
                let on = v > 0.12;
                let el = &blocks[i];
                let s = el.style();
                if on {
                    s.set_property("background", "#ffd76a").ok();
                } else {
                    s.set_property("background", "rgba(255,224,122,.12)").ok();
                }
            }
        }

        // Skip glow effect if disabled or on low-end mode
        if enable_glow {
        let glow_strength = (GLOW_BASE + energy_avg * 0.55 + bass_avg * 0.45).max(0.22).min(1.05);
        let opacity = (OUTER_GLOW_OPACITY * glow_strength).min(1.0);
        let temp = unsafe { TEMP_BUFFER.as_mut().unwrap() };
        let glow = unsafe { GLOW_BUFFER.as_mut().unwrap() };
        let prev = unsafe { PREV_FRAME.as_mut().unwrap() };
        let stride = w * 4;
        temp.fill(0);
        glow.fill(0);

        let rad = OUTER_GLOW_RADIUS.max(1) as i32;
        let win = (rad * 2 + 1) as u32;

        // Horizontal blur: frame -> glow
        for y in 0..h as i32 {
            let row_off = (y as usize) * stride;
            for x in 0..w as i32 {
                let mut sr: u32 = 0; let mut sg: u32 = 0; let mut sb: u32 = 0;
                for dx in -rad..=rad {
                    let xx = (x + dx).clamp(0, (w - 1) as i32) as usize;
                    let idx = row_off + xx * 4;
                    sr += frame[idx] as u32;
                    sg += frame[idx + 1] as u32;
                    sb += frame[idx + 2] as u32;
                }
                let o = row_off + (x as usize) * 4;
                glow[o] = (sr / win) as u8;
                glow[o + 1] = (sg / win) as u8;
                glow[o + 2] = (sb / win) as u8;
                glow[o + 3] = 255;
            }
        }

        // Vertical blur: glow -> temp
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let mut sr: u32 = 0; let mut sg: u32 = 0; let mut sb: u32 = 0;
                for dy in -rad..=rad {
                    let yy = (y + dy).clamp(0, (h - 1) as i32) as usize;
                    let idx = yy * stride + (x as usize) * 4;
                    sr += glow[idx] as u32;
                    sg += glow[idx + 1] as u32;
                    sb += glow[idx + 2] as u32;
                }
                let o = (y as usize) * stride + (x as usize) * 4;
                temp[o] = (sr / win) as u8;
                temp[o + 1] = (sg / win) as u8;
                temp[o + 2] = (sb / win) as u8;
                temp[o + 3] = 255;
            }
        }

        // Apply gamma to glow for beam-like falloff
        for i in (0..(w * h * 4)).step_by(4) {
            let r = (temp[i] as f32 / 255.0).powf(GLOW_GAMMA);
            let g = (temp[i + 1] as f32 / 255.0).powf(GLOW_GAMMA);
            let b = (temp[i + 2] as f32 / 255.0).powf(GLOW_GAMMA);
            temp[i] = (r * 255.0).min(255.0) as u8;
            temp[i + 1] = (g * 255.0).min(255.0) as u8;
            temp[i + 2] = (b * 255.0).min(255.0) as u8;
            temp[i + 3] = 255;
        }

        for i in (0..(w * h * 4)).step_by(4) {
            // slight line boost to emphasize core
            let line_boost = 1.10f32;
            let base_r = (frame[i] as f32 * line_boost).min(255.0);
            let base_g = (frame[i + 1] as f32 * line_boost).min(255.0);
            let base_b = (frame[i + 2] as f32 * line_boost).min(255.0);

            let gl_r = (temp[i] as f32) * opacity * BEAM_ADD;
            let gl_g = (temp[i + 1] as f32) * opacity * BEAM_ADD;
            let gl_b = (temp[i + 2] as f32) * opacity * BEAM_ADD;

            let out_r = (base_r + gl_r).min(255.0);
            let out_g = (base_g + gl_g).min(255.0);
            let out_b = (base_b + gl_b).min(255.0);

            frame[i] = out_r as u8;
            frame[i + 1] = out_g as u8;
            frame[i + 2] = out_b as u8;
            frame[i + 3] = 255;
        }
        } 

        // Skip temporal smoothing if disabled or on low-end mode  
        if enable_smoothing {
        let s = SMOOTHING as f32;
        let invs = 1.0 - s;
        let prev = unsafe { PREV_FRAME.as_mut().unwrap() };
        for i in (0..(w * h * 4)).step_by(4) {
            let r = frame[i] as f32 * invs + prev[i] as f32 * s;
            let gch = frame[i + 1] as f32 * invs + prev[i + 1] as f32 * s;
            let b = frame[i + 2] as f32 * invs + prev[i + 2] as f32 * s;
            let r8 = r.min(255.0) as u8;
            let g8 = gch.min(255.0) as u8;
            let b8 = b.min(255.0) as u8;
            frame[i] = r8; frame[i + 1] = g8; frame[i + 2] = b8; frame[i + 3] = 255;
            prev[i] = r8; prev[i + 1] = g8; prev[i + 2] = b8; prev[i + 3] = 255;
        }
        }

        let quiet = (energy_avg * 1.15).min(1.0);
        let darken_factor = 0.85 + 0.15 * quiet;
        if darken_factor < 0.999 {
            for i in (0..(w * h * 4)).step_by(4) {
                let r = (frame[i] as f32 * darken_factor).min(255.0);
                let g = (frame[i + 1] as f32 * darken_factor).min(255.0);
                let b = (frame[i + 2] as f32 * darken_factor).min(255.0);
                frame[i] = r as u8;
                frame[i + 1] = g as u8;
                frame[i + 2] = b as u8;
                frame[i + 3] = 255;
            }
        }

        for y in 0..h {
            if y % 3 == 0 {
                let row = y * w * 4;
                for i in (row..row + w * 4).step_by(4) {
                    frame[i] = (frame[i] as f32 * 0.88) as u8;
                    frame[i + 1] = (frame[i + 1] as f32 * 0.88) as u8;
                    frame[i + 2] = (frame[i + 2] as f32 * 0.88) as u8;
                }
            }
        }

        // Handle color inversion (wrap in unsafe)
        unsafe {
            if MOUSE_DOWN {
                if COLOR_INVERT_VALUE < 255 {
                    COLOR_INVERT_VALUE = (COLOR_INVERT_VALUE + 5).min(255);
                }
            } else if COLOR_INVERT_VALUE > 0 {
                COLOR_INVERT_VALUE = COLOR_INVERT_VALUE.saturating_sub(5);
            }
            if COLOR_INVERT_VALUE > 0 {
                soft_invert(frame, COLOR_INVERT_VALUE);
            }
        }
}