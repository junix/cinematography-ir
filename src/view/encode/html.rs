//! HTML encoder. Stills embed the SVG; animations play the sampled cells on
//! the scene's own time base and implement the `insight-video` composition
//! contract (`<main data-composition-id data-duration>` in seconds plus
//! `window.__insightVideo.seek(t)`), so
//! `insight-video render page.html -o clip.mp4` yields a deterministic video
//! (ADR-1102, ADR-1115 D3).

use std::fmt::Write;

use crate::view::canvas::Canvas;
use crate::view::encode::svg::canvas_to_inline_svg;

fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn still_page(title: &str, canvas: &Canvas) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>{}</title>\
<style>body{{margin:0;background:#f4f4f6;display:flex;justify-content:center;align-items:center;min-height:100vh}}\
main{{width:min(100vw,{}px)}}svg{{width:100%;height:auto;display:block;background:#fff}}</style></head>\
<body><main data-composition-id=\"{}\">{}</main></body></html>\n",
        esc(title),
        canvas.width.round() as i64,
        esc(title),
        canvas_to_inline_svg(canvas)
    )
}

/// One page playing `frames` — each tagged with the scene time (seconds) at
/// which it becomes current — over `duration_s`. Autoplays for humans;
/// `seek(t)` stops autoplay and shows the frame current at `t`, which is what
/// insight-video calls before every capture. `fps_hint` is informational.
pub fn animation_page(
    title: &str,
    frames: &[(f64, &Canvas)],
    duration_s: f64,
    fps_hint: f64,
) -> String {
    let count = frames.len().max(1);
    let width = frames.first().map_or(800.0, |(_, c)| c.width).round() as i64;
    let mut out = String::with_capacity(frames.len() * 4096);
    let _ = write!(
        out,
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>{}</title>\
<style>body{{margin:0;background:#f4f4f6;display:flex;justify-content:center;align-items:center;min-height:100vh}}\
main{{width:min(100vw,{width}px);position:relative}}.frame{{display:none}}.frame.active{{display:block}}\
svg{{width:100%;height:auto;display:block;background:#fff}}</style></head>\
<body><main data-composition-id=\"{}\" data-duration=\"{duration_s}\" data-fps=\"{fps_hint}\" data-frames=\"{count}\">",
        esc(title),
        esc(title)
    );
    for (index, (time, frame)) in frames.iter().enumerate() {
        let active = if index == 0 { " active" } else { "" };
        let _ = write!(
            out,
            "<div class=\"frame{active}\" data-frame=\"{index}\" data-t=\"{time}\">{}</div>",
            canvas_to_inline_svg(frame)
        );
    }
    let times: Vec<String> = frames.iter().map(|(t, _)| t.to_string()).collect();
    let _ = write!(
        out,
        "</main><script>(function(){{\
var frames=Array.prototype.slice.call(document.querySelectorAll('.frame'));\
var times=[{}];var duration={duration_s};var count=frames.length;var current=0;var timer=null;\
function frameAt(t){{var i=0;for(var k=0;k<times.length;k++){{if(times[k]<=t+1e-6)i=k;else break;}}return i;}}\
function show(i){{i=Math.max(0,Math.min(count-1,i));if(i===current)return;frames[current].classList.remove('active');frames[i].classList.add('active');current=i;}}\
function stop(){{if(timer!==null){{clearInterval(timer);timer=null;}}}}\
window.__insightVideo={{duration:duration,ready:true,seek:function(t){{stop();show(frameAt(t));}}}};\
window.__cineFrames={{count:count,times:times,duration:duration,show:show,stop:stop,frameAt:frameAt}};\
var started=Date.now();\
timer=setInterval(function(){{var t=((Date.now()-started)/1000)%Math.max(duration,1e-3);show(frameAt(t));}},40);\
}})();</script></body></html>",
        times.join(",")
    );
    out.push('\n');
    out
}
