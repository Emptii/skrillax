use crate::comp::player::{Agent, Buffed, Inventory, MovementState, Player};
use crate::comp::pos::{Heading, LocalPosition, Position};
use crate::comp::sync::Synchronize;
use crate::comp::visibility::Visibility;
use crate::comp::{CharacterSelect, Client, GameEntity, Playing};
use crate::db::character::{CharacterData, CharacterItem};
use crate::ext::AsyncTaskCreate;
use crate::login::character_loader::{check_name_available, create_character, load_characters_sparse, Character};
use crate::login::job_distribution::JobDistribution;
use crate::server_plugin::ServerId;
use crate::time::AsSilkroadTime;
use crate::world::id_allocator::IdAllocator;
use crate::GameSettings;
use bevy_ecs::prelude::*;
use cgmath::Vector3;
use chrono::{TimeZone, Utc};
use silkroad_protocol::character::{
    CharacterJoinRequest, CharacterJoinResponse, CharacterJoinResult, CharacterListAction, CharacterListContent,
    CharacterListEntry, CharacterListEquippedItem, CharacterListError, CharacterListRequest,
    CharacterListRequestAction, CharacterListResponse, CharacterListResult,
};
use silkroad_protocol::world::{
    ActionState, AliveState, BodyState, CharacterSpawn, CharacterSpawnEnd, CharacterSpawnStart, EntityState,
    InventoryItemBindingData, InventoryItemContentData, InventoryItemData, JobType,
};
use silkroad_protocol::{ClientPacket, ServerPacket};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::oneshot::error::TryRecvError;
use tracing::warn;

pub(crate) fn charselect(
    settings: Res<GameSettings>,
    job_distribution: Res<JobDistribution>,
    pool: Res<PgPool>,
    task_creator: Res<Arc<Runtime>>,
    server_id: Res<ServerId>,
    mut cmd: Commands,
    mut allocator: ResMut<IdAllocator>,
    mut query: Query<(Entity, &mut Client, &mut CharacterSelect, &Playing)>,
) {
    for (entity, mut client, mut character_list, playing) in query.iter_mut() {
        let mut character_list: &mut CharacterSelect = &mut character_list;
        while let Some(packet) = client.1.pop_front() {
            match packet {
                ClientPacket::CharacterListRequest(CharacterListRequest { action }) => match action {
                    CharacterListRequestAction::Create {
                        character_name,
                        ref_id,
                        scale,
                        chest,
                        pants,
                        boots,
                        weapon,
                    } => {
                        let character = create_character_from(
                            playing.0.id,
                            server_id.0,
                            character_name,
                            ref_id,
                            scale,
                            chest,
                            pants,
                            boots,
                            weapon,
                        );
                        let task = task_creator.create_task(create_character(pool.clone(), character));
                        character_list.character_create = Some(task);
                    },
                    CharacterListRequestAction::List => {
                        if character_list.character_receiver.is_some() {
                            continue;
                        }

                        let receiver =
                            task_creator.create_task(load_characters_sparse(pool.clone(), playing.0.id, server_id.0));
                        character_list.character_receiver = Some(receiver);
                    },
                    CharacterListRequestAction::Delete { .. } => {},
                    CharacterListRequestAction::CheckName { character_name } => {
                        if character_list.character_name_check.is_none() {
                            let task = task_creator.create_task(check_name_available(
                                pool.clone(),
                                character_name,
                                server_id.0,
                            ));
                            character_list.character_name_check = Some(task);
                        }
                    },
                    CharacterListRequestAction::Restore { .. } => {},
                    CharacterListRequestAction::ShowJobSpread => {
                        let (hunter_perc, thief_perc) = job_distribution.spread();
                        send_job_spread(&client, hunter_perc, thief_perc);
                    },
                    CharacterListRequestAction::AssignJob { .. } => {},
                },
                ClientPacket::CharacterJoinRequest(CharacterJoinRequest { character_name }) => {
                    match character_list.characters {
                        Some(ref characters) => {
                            let character = characters
                                .iter()
                                .find(|char| char.character_data.charname == character_name.as_ref())
                                .unwrap();

                            let player = Player {
                                user: playing.0.clone(),
                                character: crate::comp::player::Character::from_db_character(&character.character_data),
                                inventory: Inventory::from(&character.items, 45),
                                logout: None,
                            };

                            let data = &character.character_data;

                            let position = Position {
                                location: LocalPosition(
                                    (data.region as u16).into(),
                                    Vector3::new(data.x, data.y, data.z),
                                )
                                .to_global(),
                                rotation: Heading(0.0),
                            };

                            let agent = Agent {
                                movement_speed: 50.0,
                                movement_state: MovementState::Standing,
                                movement_target: None,
                            };

                            let game_entity = GameEntity {
                                ref_id: data.character_type as u32,
                                unique_id: allocator.allocate(),
                            };

                            client.send(CharacterJoinResponse::new(CharacterJoinResult::Success));

                            send_spawn(&client, &game_entity, &player, &position, settings.max_level);

                            cmd.entity(entity)
                                .insert(Synchronize::default())
                                .insert(game_entity)
                                .insert(player)
                                .insert(agent)
                                .insert(position.clone())
                                .insert(Buffed {})
                                .insert(Visibility::with_radius(200.))
                                .remove::<CharacterSelect>();
                        },
                        None => {
                            client.send(CharacterJoinResponse::new(CharacterJoinResult::Error {
                                error: CharacterListError::ReachedCapacity, // TODO
                            }));
                        },
                    }
                },
                _ => {},
            }
        }

        if let Some(receiver) = character_list.character_receiver.as_mut() {
            match receiver.try_recv() {
                Ok(characters) => {
                    send_character_list(&client, &characters);
                    character_list.characters = Some(characters);
                    character_list.character_receiver = None;
                },
                Err(TryRecvError::Empty) => {},
                Err(_) => {
                    warn!(id = playing.0.id, "Error when loading characters.");
                    character_list.character_receiver = None;
                },
            }
        }

        if let Some(receiver) = character_list.character_name_check.as_mut() {
            match receiver.try_recv() {
                Ok(available) => {
                    let result = if available {
                        CharacterListResult::ok(CharacterListContent::Empty)
                    } else {
                        CharacterListResult::error(CharacterListError::NameAlreadyUsed)
                    };
                    client.send(ServerPacket::CharacterListResponse(CharacterListResponse::new(
                        CharacterListAction::CheckName,
                        result,
                    )));
                    character_list.character_name_check = None;
                },
                Err(TryRecvError::Empty) => {},
                Err(_) => {
                    warn!(id = playing.0.id, "Error when checking name.");
                    character_list.character_name_check = None;
                },
            }
        }

        if let Some(receiver) = character_list.character_create.as_mut() {
            match receiver.try_recv() {
                Ok(_) => {
                    client.send(ServerPacket::CharacterListResponse(CharacterListResponse::new(
                        CharacterListAction::Create,
                        CharacterListResult::ok(CharacterListContent::Empty),
                    )));
                    character_list.character_create = None;
                },
                Err(TryRecvError::Empty) => {},
                Err(_) => {
                    warn!(id = playing.0.id, "Error when creating character.");
                    character_list.character_create = None;
                },
            }
        }
    }
}

fn send_character_list(client: &Client, character_list: &Vec<Character>) {
    let characters = character_list.iter().map(|chara| from_character(chara)).collect();
    let response = CharacterListResponse::new(
        CharacterListAction::List,
        CharacterListResult::ok(CharacterListContent::characters(characters, 0)),
    );
    client.send(response);
}

fn from_character(character: &Character) -> CharacterListEntry {
    let data = &character.character_data;
    CharacterListEntry {
        ref_id: data.character_type as u32,
        name: data.charname.clone(),
        unknown_1: 0,
        unknown_2: 0,
        scale: data.scale as u8,
        level: data.levels as u8,
        exp: data.exp as u64,
        sp: data.sp as u32,
        strength: data.strength as u16,
        intelligence: data.intelligence as u16,
        stat_points: data.stat_points as u16,
        hp: data.current_hp as u32,
        mp: data.current_mp as u32,
        remaining_deletion_time: None,
        region: data.region as u16,
        last_logout: data.last_logout.map(|time| time.as_silkroad_time()).unwrap_or(0),
        guild_member_class: 0,
        guild_rename_required: None,
        academy_member_class: 0,
        equipped_items: character.items.iter().map(|item| from_item(item)).collect(),
        avatar_items: Vec::new(),
    }
}

fn from_item(item: &CharacterItem) -> CharacterListEquippedItem {
    CharacterListEquippedItem::new(item.item_obj_id as u32, item.upgrade_level as u8)
}

fn send_job_spread(client: &Client, hunters: u8, thieves: u8) {
    client.send(CharacterListResponse::new(
        CharacterListAction::ShowJobSpread,
        CharacterListResult::ok(CharacterListContent::jobspread(hunters, thieves)),
    ));
}

fn send_spawn(client: &Client, entity: &GameEntity, player: &Player, position: &Position, max_level: u8) {
    client.send(CharacterSpawnStart);

    let character_data = &player.character;

    let entity_state = EntityState {
        alive: AliveState::Spawning,
        unknown1: 0,
        action_state: ActionState::None,
        body_state: BodyState::None,
        unknown2: 0,
        walk_speed: 16.0,
        run_speed: 50.0,
        berserk_speed: 100.0,
        active_buffs: vec![],
    };

    let inventory_items = player
        .inventory
        .items()
        .map(|(slot, item)| InventoryItemData {
            slot: *slot,
            rent_data: 0,
            item_id: item.ref_id as u32,
            content_data: InventoryItemContentData::Equipment {
                plus_level: item.upgrade_level,
                variance: item.variance.unwrap_or_default(),
                durability: 1,
                magic: vec![],
                bindings_1: InventoryItemBindingData::new(1, 0),
                bindings_2: InventoryItemBindingData::new(2, 0),
                bindings_3: InventoryItemBindingData::new(3, 0),
                bindings_4: InventoryItemBindingData::new(4, 0),
            },
        })
        .collect();

    client.send(CharacterSpawn::new(
        Utc::now().as_silkroad_time(),
        entity.ref_id,
        character_data.scale,
        character_data.level,
        character_data.max_level,
        character_data.exp,
        character_data.sp_exp,
        character_data.gold,
        character_data.sp,
        character_data.stat_points,
        character_data.berserk_points,
        character_data.current_hp,
        character_data.current_mp,
        character_data.beginner_mark,
        0,
        0,
        0,
        0,
        0,
        0x4,
        Utc.ymd(2000, 1, 1).and_hms(0, 0, 0),
        0,
        max_level,
        player.inventory.size() as u8,
        inventory_items,
        5,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        entity.unique_id,
        position.as_protocol(),
        0,
        position.rotation.into(),
        entity_state,
        character_data.name.clone(),
        String::new(),
        JobType::None, // TODO
        1,
        0,
        0,
        0,
        0,
        false,
        0,
        0xFF,
        2,
        character_data.gm,
        Vec::new(),
        0,
        1,
        1,
        2,
        Vec::new(),
    ));

    client.send(CharacterSpawnEnd);
}

pub(crate) fn create_character_from(
    user_id: i32,
    server_id: u16,
    character_name: String,
    ref_id: u32,
    scale: u8,
    chest: u32,
    pants: u32,
    boots: u32,
    weapon: u32,
) -> Character {
    let character = CharacterData {
        id: 0,
        user_id,
        server_id: server_id as i32,
        charname: character_name,
        character_type: ref_id as i32,
        scale: scale as i16,
        levels: 1,
        max_level: 1,
        exp: 0,
        sp: 0,
        sp_exp: 0,
        strength: 20,
        intelligence: 20,
        stat_points: 0,
        current_hp: 200,
        current_mp: 200,
        deletion_started: None,
        x: 739.,
        y: 37.4519,
        z: 1757.,
        region: 24998,
        berserk_points: 0,
        gold: 5000000,
        beginner_mark: true,
        gm: false,
        last_logout: None,
    };

    let items = vec![
        CharacterItem {
            id: 0,
            character_id: 0,
            item_obj_id: chest as i32,
            upgrade_level: 0,
            variance: None,
            slot: 1,
        },
        CharacterItem {
            id: 0,
            character_id: 0,
            item_obj_id: pants as i32,
            upgrade_level: 0,
            variance: None,
            slot: 4,
        },
        CharacterItem {
            id: 0,
            character_id: 0,
            item_obj_id: boots as i32,
            upgrade_level: 0,
            variance: None,
            slot: 5,
        },
        CharacterItem {
            id: 0,
            character_id: 0,
            item_obj_id: weapon as i32,
            upgrade_level: 0,
            variance: None,
            slot: 6,
        },
    ];
    Character {
        character_data: character,
        items,
    }
}