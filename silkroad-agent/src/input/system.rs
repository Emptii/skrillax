use crate::comp::net::{Client, LastAction};
use crate::config::GameConfig;
use crate::event::{ClientDisconnectedEvent, LoadingFinishedEvent};
use crate::input::{LoginInput, PlayerInput};
use crate::mall::event::MallOpenRequestEvent;
use bevy_ecs::prelude::*;
use bevy_time::{Real, Time};
use silkroad_game_base::StatType;
use silkroad_network::stream::{SendResult, Stream, StreamError};
use silkroad_protocol::character::GameGuideResponse;
use silkroad_protocol::general::IdentityInformation;
use silkroad_protocol::inventory::ConsignmentResponse;
use silkroad_protocol::ClientPacket;
use std::time::Instant;
use tracing::warn;

pub(crate) fn reset(mut player_input: Query<&mut PlayerInput>, mut login_input: Query<&mut LoginInput>) {
    for mut input in player_input.iter_mut() {
        input.reset();
    }

    for mut input in login_input.iter_mut() {
        input.reset();
    }
}

pub(crate) fn receive_game_inputs(
    mut query: Query<(Entity, &Client, &mut PlayerInput, &mut LastAction)>,
    time: Res<Time<Real>>,
    settings: Res<GameConfig>,
    mut loading_events: EventWriter<LoadingFinishedEvent>,
    mut disconnect_events: EventWriter<ClientDisconnectedEvent>,
    mut mall_events: EventWriter<MallOpenRequestEvent>,
) {
    for (entity, client, mut input, mut last_action) in query.iter_mut() {
        let mut had_action = false;
        loop {
            match client.received() {
                Ok(Some(packet)) => {
                    had_action = true;
                    match packet {
                        ClientPacket::ChatMessage(chat) => {
                            input.chat.push(*chat);
                        },
                        ClientPacket::Rotation(rotate) => {
                            input.rotation = Some(*rotate);
                        },
                        ClientPacket::PlayerMovementRequest(request) => {
                            input.movement = Some(request.kind);
                        },
                        ClientPacket::LogoutRequest(logout) => {
                            input.logout = Some(*logout);
                        },
                        ClientPacket::TargetEntity(target) => {
                            input.target = Some(*target);
                        },
                        ClientPacket::UnTargetEntity(untarget) => {
                            input.untarget = Some(*untarget);
                        },
                        ClientPacket::PerformAction(action) => {
                            input.action = Some(*action);
                        },
                        ClientPacket::GmCommand(command) => {
                            input.gm = Some(*command);
                        },
                        ClientPacket::OpenItemMall(_) => {
                            mall_events.send(MallOpenRequestEvent(entity));
                        },
                        ClientPacket::InventoryOperation(inventory) => {
                            input.inventory = Some(*inventory);
                        },
                        ClientPacket::ConsignmentList(_) => {
                            client.send(ConsignmentResponse::success_empty());
                        },
                        ClientPacket::AddFriend(_) => {},
                        ClientPacket::CreateFriendGroup(_) => {},
                        ClientPacket::DeleteFriend(_) => {},
                        ClientPacket::UpdateGameGuide(guide) => {
                            client.send(GameGuideResponse::Success(guide.0));
                        },
                        ClientPacket::FinishLoading(_) => {
                            loading_events.send(LoadingFinishedEvent(entity));
                        },
                        ClientPacket::LevelUpMastery(mastery) => {
                            input.mastery = Some(*mastery);
                        },
                        ClientPacket::LearnSkill(skill) => input.skill_add = Some(*skill),
                        ClientPacket::IncreaseStr(_) => input.increase_stats.push(StatType::STR),
                        ClientPacket::IncreaseInt(_) => input.increase_stats.push(StatType::INT),
                        _ => {},
                    }
                },
                Ok(None) => {
                    break;
                },
                Err(StreamError::StreamClosed) => {
                    disconnect_events.send(ClientDisconnectedEvent(entity));
                    break;
                },
                Err(e) => {
                    warn!(id = ?client.0.id(), "Error when receiving. {:?}", e);
                },
            }
        }

        let last_tick_time = time.last_update().unwrap_or_else(Instant::now);
        if had_action {
            last_action.0 = last_tick_time;
        }

        if last_tick_time.duration_since(last_action.0).as_secs() > settings.client_timeout.into() {
            disconnect_events.send(ClientDisconnectedEvent(entity));
        }
    }
}

pub(crate) fn receive_login_inputs(
    mut query: Query<(Entity, &Client, &mut LoginInput, &mut LastAction)>,
    time: Res<Time<Real>>,
    settings: Res<GameConfig>,
    mut disconnect_events: EventWriter<ClientDisconnectedEvent>,
) {
    for (entity, client, mut input, mut last_action) in query.iter_mut() {
        let mut had_action = false;
        loop {
            match client.received() {
                Ok(Some(packet)) => {
                    had_action = true;
                    match packet {
                        ClientPacket::CharacterListRequest(request) => {
                            input.list.push(request.action);
                        },
                        ClientPacket::CharacterJoinRequest(join) => {
                            input.join = Some(*join);
                        },
                        ClientPacket::AuthRequest(auth) => {
                            input.auth = Some(*auth);
                        },
                        ClientPacket::IdentityInformation(_id) => send_identity_information(client).unwrap(),
                        _ => {},
                    }
                },
                Ok(None) => {
                    break;
                },
                Err(StreamError::StreamClosed) => {
                    disconnect_events.send(ClientDisconnectedEvent(entity));
                    break;
                },
                Err(e) => {
                    warn!(id = ?client.0.id(), "Error when receiving. {:?}", e);
                },
            }
        }

        let last_tick_time = time.last_update().unwrap_or_else(Instant::now);
        if had_action {
            last_action.0 = last_tick_time;
        }

        if last_tick_time.duration_since(last_action.0).as_secs() > settings.client_timeout.into() {
            disconnect_events.send(ClientDisconnectedEvent(entity));
        }
    }
}

fn send_identity_information(client: &Stream) -> SendResult {
    client.send(IdentityInformation::new("AgentServer".to_string(), 0))
}
