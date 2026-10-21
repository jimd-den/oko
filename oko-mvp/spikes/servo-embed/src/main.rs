//! Spike: headless Servo via the published crate, no display server.
//!
//! The delegate drives WebView::paint on notify_new_frame_ready; the first
//! frame is taken with take_screenshot, which waits for load and layout. The
//! paint loop is required: screenshot with an empty delegate timed out on CI.

use std::cell::Cell;
use std::error::Error;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use servo::{
    EventLoopWaker, RenderingContext, ServoBuilder, SoftwareRenderingContext, WebView,
    WebViewBuilder, WebViewDelegate,
};
use url::Url;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 200;

struct Waker(mpsc::Sender<()>);

impl EventLoopWaker for Waker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(Self(self.0.clone()))
    }
    fn wake(&self) {
        let _ = self.0.send(());
    }
}

struct Delegate {
    dirty: Rc<Cell<bool>>,
    wake: mpsc::Sender<()>,
}

impl WebViewDelegate for Delegate {
    fn notify_new_frame_ready(&self, _: WebView) {
        self.dirty.set(true);
        let _ = self.wake.send(());
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .ok();

    let (wake_tx, wake_rx) = mpsc::channel();
    let servo = ServoBuilder::default()
        .event_loop_waker(Box::new(Waker(wake_tx.clone())))
        .build();
    servo.setup_logging();

    let size = dpi::PhysicalSize::new(WIDTH, HEIGHT);
    let context = Rc::new(
        SoftwareRenderingContext::new(size)
            .map_err(|e| std::io::Error::other(format!("context: {:?}", e)))?,
    );
    let _ = context.make_current();

    let dirty = Rc::new(Cell::new(false));
    let url = Url::parse(
        "data:text/html,<body style=background:%23101828;color:%23ff9c00>\
         <h1>servo crate</h1></body>",
    )?;
    let webview = WebViewBuilder::new(&servo, context.clone())
        .url(url)
        .delegate(Rc::new(Delegate {
            dirty: dirty.clone(),
            wake: wake_tx,
        }))
        .build();

    let (shot_tx, shot_rx) = mpsc::channel();
    webview.take_screenshot(None, move |result| {
        let _ = shot_tx.send(result);
    });

    let start = Instant::now();
    let deadline = start + Duration::from_secs(30);
    let mut forced = false;
    let mut image = None;
    while Instant::now() < deadline {
        servo.spin_event_loop();
        if !forced && start.elapsed() > Duration::from_secs(1) {
            dirty.set(true);
            forced = true;
        }
        if dirty.replace(false) {
            webview.paint();
        }
        if let Ok(result) = shot_rx.try_recv() {
            image = Some(
                result.map_err(|e| std::io::Error::other(format!("screenshot: {:?}", e)))?,
            );
            break;
        }
        let _ = wake_rx.recv_timeout(Duration::from_millis(10));
    }

    let image = image.ok_or("no frame after 30s")?;
    let mut ppm = format!("P6\n{} {}\n255\n", image.width(), image.height()).into_bytes();
    for pixel in image.pixels() {
        ppm.extend_from_slice(&pixel.0[0..3]);
    }
    std::fs::write("frame.ppm", ppm)?;
    println!("wrote {}x{} frame to frame.ppm", image.width(), image.height());
    Ok(())
}
