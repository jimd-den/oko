//! Spike: headless Servo via the published crate, no display server.
//!
//! Creates a SoftwareRenderingContext, loads a data URL in a WebView, waits for
//! a screenshot, and writes a PPM. This is the API shape the real
//! oko-servo-embed process will use; it is a probe, not a production client.

use std::error::Error;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use servo::{
    EventLoopWaker, ServoBuilder, SoftwareRenderingContext, WebView, WebViewBuilder,
    WebViewDelegate,
};
use url::Url;

struct Waker(mpsc::Sender<()>);

impl EventLoopWaker for Waker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(Self(self.0.clone()))
    }
    fn wake(&self) {
        let _ = self.0.send(());
    }
}

struct Delegate;

impl WebViewDelegate for Delegate {
    fn notify_new_frame_ready(&self, _: WebView) {}
}

fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .ok();

    let (wake_tx, wake_rx) = mpsc::channel();
    let servo = ServoBuilder::default()
        .event_loop_waker(Box::new(Waker(wake_tx)))
        .build();
    servo.setup_logging();

    let size = dpi::PhysicalSize::new(320, 200);
    let context = Rc::new(SoftwareRenderingContext::new(size)?);
    let _ = context.make_current();

    let url = Url::parse(
        "data:text/html,<body style=background:%23101828;color:%23ff9c00>\
         <h1>servo crate</h1></body>",
    )?;
    let webview = WebViewBuilder::new(&servo, context.clone())
        .url(url)
        .delegate(Rc::new(Delegate))
        .build();

    let (shot_tx, shot_rx) = mpsc::channel();
    webview.take_screenshot(None, move |result| {
        let _ = shot_tx.send(result);
    });

    let mut image = None;
    for _ in 0..600 {
        servo.spin_event_loop();
        if let Ok(result) = shot_rx.try_recv() {
            image = Some(result?);
            break;
        }
        let _ = wake_rx.recv_timeout(Duration::from_millis(10));
    }

    let image = image.ok_or("screenshot timed out")?;
    let mut ppm = format!("P6\n{} {}\n255\n", image.width(), image.height()).into_bytes();
    for pixel in image.pixels() {
        ppm.extend_from_slice(&pixel.0[0..3]);
    }
    std::fs::write("frame.ppm", ppm)?;
    println!("wrote {}x{} frame to frame.ppm", image.width(), image.height());
    Ok(())
}
