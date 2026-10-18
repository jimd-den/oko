//! Message dispatch — routes IPC messages to handlers.
//! Small composable route matching (SRD §4.2).

use crate::ipc::json::object;
use crate::ipc::message::{Direction, Message, IPC_VERSION};
use crate::ipc::server::IpcServer;
use crate::ipc::transport::IpcTransport;

use crate::ipc::server::handlers::{
    app, blink, browser, render, status, terminal, ui,
};

pub fn handle_message<T: IpcTransport>(server: &mut IpcServer<T>, msg: &Message) -> Message {
    if msg.version != IPC_VERSION {
        return Message::error_response(&msg.id, &msg.route, "protocol_error", "bad version");
    }
    let reply_route = format!("{}.reply", msg.route);

    let result = match msg.direction {
        Direction::Request => dispatch_request(server, msg),
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

fn dispatch_request<T: IpcTransport>(server: &mut IpcServer<T>, msg: &Message) -> Result<crate::ipc::json::JsonValue, String> {
    match msg.route.as_str() {
        "host.status" => status::handle(server),
        "host.terminal.create" => terminal::handle_create(server),
        "host.terminal.read" => terminal::handle_read(server, &msg.payload),
        "host.app.list" => app::handle_list(server),
        "host.app.launch" => app::handle_launch(server, &msg.payload),
        "host.app.stop" => app::handle_stop(server, &msg.payload),
        "host.browser.open" => browser::handle_open(server, &msg.payload),
        "host.browser.engine" => browser::handle_engine(server),
        "host.browser.capabilities" => browser::handle_capabilities(server),
        "blink.install" => blink::handle_install(server),
        "blink.status" => blink::handle_status(server),
        "blink.fetch-checksum" => blink::handle_fetch_checksum(server, &msg.payload),
        "ui.mode.desktop" => ui::handle_desktop(server),
        "render.demo" => render::handle_demo(server, &msg.payload),
        "render.stats" => render::handle_stats(server),
        "render.capture" => render::handle_capture(server, &msg.payload),
        "host.shutdown" => Ok(object(vec![])),
        _ => Err(format!("unknown route: {}", msg.route)),
    }
}
