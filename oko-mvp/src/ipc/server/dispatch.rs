//! Message dispatch — routes IPC messages to handlers.
//! Small composable route matching (SRD §4.2).

use crate::ipc::json::object;
use crate::ipc::message::{Direction, Message, IPC_VERSION};
use crate::ipc::server::Core;

use crate::ipc::server::capabilities;
use crate::ipc::server::handlers::{app, app_new, blink, browser, computer, fs, render, status, terminal, ui};

pub fn handle_message(core: &mut Core, msg: &Message) -> Message {
    if msg.version != IPC_VERSION {
        return Message::error_response(&msg.id, &msg.route, "protocol_error", "bad version");
    }
    let reply_route = format!("{}.reply", msg.route);

    let result = match msg.direction {
        Direction::Request => dispatch_request(core, msg),
        Direction::Response => Ok(object(vec![("acked".into(), crate::ipc::json::bool(true))])),
        Direction::Notification => {
            // e.g. "ui.reload" — core keeps sessions alive (TERM-003).
            Ok(object(vec![("acked".into(), crate::ipc::json::bool(true))]))
        }
    };

    match result {
        Ok(payload) => Message::response(&msg.id, &reply_route, payload),
        Err(e) => Message::error_response(&msg.id, &msg.route, "host_error", &e),
    }
}

fn dispatch_request(core: &mut Core, msg: &Message) -> Result<crate::ipc::json::JsonValue, String> {
    // Enforced here, before any handler runs, so a route cannot forget its own
    // check and a caller cannot grant itself anything (SRD SEC-002, §8.3).
    let required = capabilities::required_for(&msg.route);
    if !core.policy.allows(&required) {
        return Err(capabilities::denial(&msg.route, &required));
    }
    match msg.route.as_str() {
        "host.status" => status::handle(core),
        "computer.ask" => computer::handle_ask(core, &msg.payload),
        "host.terminal.create" => terminal::handle_create(core),
        "host.terminal.read" => terminal::handle_read(core, &msg.payload),
        "host.terminal.write" => terminal::handle_write(core, &msg.payload),
        "host.app.list" => app::handle_list(core),
        "host.app.create" => app_new::handle_create(core, &msg.payload),
        "host.fs.list" => fs::handle_list(core, &msg.payload),
        "host.fs.read" => fs::handle_read(core, &msg.payload),
        "host.fs.write" => fs::handle_write(core, &msg.payload),
        "host.app.launch" => app::handle_launch(core, &msg.payload),
        "host.app.stop" => app::handle_stop(core, &msg.payload),
        "host.browser.open" => browser::handle_open(core, &msg.payload),
        "host.browser.engine" => browser::handle_engine(core),
        "host.browser.capabilities" => browser::handle_capabilities(core),
        "blink.install" => blink::handle_install(core),
        "blink.status" => blink::handle_status(core),
        "blink.fetch-checksum" => blink::handle_fetch_checksum(core, &msg.payload),
        "ui.mode.desktop" => ui::handle_desktop(core),
        "ui.pane.open" => ui::handle_pane_open(core, &msg.payload),
        "render.demo" => render::handle_demo(core, &msg.payload),
        "render.stats" => render::handle_stats(core),
        "render.capture" => render::handle_capture(core, &msg.payload),
        "host.shutdown" => Ok(object(vec![])),
        _ => Err(format!("unknown route: {}", msg.route)),
    }
}
