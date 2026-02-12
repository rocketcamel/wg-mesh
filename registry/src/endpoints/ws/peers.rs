use crate::{AppState, error::Error, storage::StorageImpl};
use actix_web::{HttpRequest, HttpResponse, rt, web};
use actix_ws::AggregatedMessage;
use futures_util::StreamExt;
use registry::PeerMessage;

pub async fn peers(
    req: HttpRequest,
    stream: web::Payload,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let (res, mut session, msg_stream) =
        actix_ws::handle(&req, stream).map_err(|e| Error::ws(e))?;

    let mut msg_stream = msg_stream.aggregate_continuations();

    let mut peer_rx = app_state.peer_updates.subscribe();

    match app_state.storage.get_peers().await {
        Ok(initial_peers) => {
            let msg = PeerMessage::HydratePeers {
                peers: initial_peers.clone(),
            };
            let json = serde_json::to_string(&msg)
                .unwrap_or_else(|_| r#"{"type":"HydratePeers","peers":[]}"#.to_string());
            if session.text(json).await.is_err() {
                return Ok(res);
            }
            tracing::info!(
                "sent initial peer list ({} peers) to new client",
                initial_peers.len()
            );
        }
        Err(e) => {
            tracing::warn!("failed to fetch initial peers: {:?}", e);
            session.close(None).await.ok();
            return Ok(res);
        }
    }

    rt::spawn(async move {
        loop {
            tokio::select! {
                msg = msg_stream.next() => {
                    match msg {
                        Some(Ok(AggregatedMessage::Ping(data))) => {
                            if session.pong(&data).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(AggregatedMessage::Pong(_))) => {}
                        Some(Ok(AggregatedMessage::Close(_))) => {
                            break;
                        }
                        Some(Ok(AggregatedMessage::Text(_))) => {
                        }
                        Some(Ok(AggregatedMessage::Binary(_))) => {
                        }
                        Some(Err(_)) => {
                            break;
                        }
                        None => {
                            break;
                        }
                    }
                }
                update = peer_rx.recv() => {
                    match update {
                        Ok(peer_update) => {
                            let json = serde_json::to_string(&peer_update.peer)
                                .unwrap_or_else(|_| "{}".to_string());
                            if session.text(json).await.is_err() {
                                break;
                            }
                            tracing::info!("sent peer update to client: {}", peer_update.peer.public_key);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("client lagged, missed {} updates", n);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
            }
        }
        session.close(None).await.ok();
        tracing::info!("client disconnected");
    });

    Ok(res)
}
