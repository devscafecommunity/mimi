use actix::{Actor, ActorContext, StreamHandler};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde_json::json;

use super::types::WsMessage;

/// WebSocket session handler
pub struct WsSession {
    channel: String,
}

impl WsSession {
    /// Create new session
    pub fn new(channel: &str) -> Self {
        WsSession {
            channel: channel.to_string(),
        }
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => match serde_json::from_str::<WsMessage>(&text) {
                Ok(_msg) => {
                    let response = json!({
                        "event_type": "connected",
                        "channel": self.channel
                    });
                    ctx.text(response.to_string());
                },
                Err(_) => {
                    let error = json!({
                        "event_type": "error",
                        "message": "Invalid JSON"
                    });
                    ctx.text(error.to_string());
                },
            },
            Ok(ws::Message::Close(_)) => {
                ctx.stop();
            },
            _ => {},
        }
    }
}

/// WebSocket endpoint: /ws/execute - Stream task execution
pub async fn ws_execute_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    ws::start(WsSession::new("execute"), &req, stream)
}

/// WebSocket endpoint: /ws/query - Stream query results
pub async fn ws_query_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    ws::start(WsSession::new("query"), &req, stream)
}

/// WebSocket endpoint: /ws/health - Health check over WebSocket
pub async fn ws_health_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    ws::start(WsSession::new("health"), &req, stream)
}
