use crate::comp::player::Player;
use crate::comp::sync::{MovementUpdate, Synchronize};
use crate::comp::visibility::Visibility;
use crate::comp::{Client, GameEntity};
use bevy_ecs::prelude::*;
use silkroad_protocol::world::{
    EntityDespawn, GroupEntitySpawnData, GroupEntitySpawnEnd, GroupEntitySpawnStart, GroupSpawnDataContent,
    GroupSpawnType, MovementDestination, MovementSource, PlayerMovementResponse,
};
use silkroad_protocol::ServerPacket;

pub(crate) fn sync_changes_others(
    query: Query<(&Client, &Visibility), With<Player>>,
    others: Query<(&GameEntity, &Synchronize)>,
) {
    for (client, visibility) in query.iter() {
        let client: &Client = client;
        let visibility: &Visibility = visibility;

        for (entity, synchronize) in visibility
            .entities_in_radius
            .iter()
            .map(|entity| others.get(*entity))
            .filter_map(|res| res.ok())
        {
            let entity: &GameEntity = entity;
            let synchronize: &Synchronize = synchronize;
            if let Some(movement) = &synchronize.movement {
                update_movement_for(client, entity, movement);
            }
        }
    }
}

fn update_movement_for(client: &Client, entity: &GameEntity, movement: &MovementUpdate) {
    match movement {
        MovementUpdate::StartMove(current, target) => {
            client.send(ServerPacket::PlayerMovementResponse(PlayerMovementResponse::new(
                entity.unique_id,
                MovementDestination::location(target.0.id(), target.1.x as u16, target.1.y as u16, target.1.z as u16),
                Some(MovementSource::new(
                    current.0.id(),
                    (current.1.x * 10.) as u16,
                    current.1.y * 10.,
                    (current.1.z * 10.) as u16,
                )),
            )));
        },
        MovementUpdate::StopMove(current) => {
            client.send(ServerPacket::PlayerMovementResponse(PlayerMovementResponse::new(
                entity.unique_id,
                MovementDestination::location(
                    current.0.id(),
                    current.1.x as u16,
                    current.1.y as u16,
                    current.1.z as u16,
                ),
                None,
            )));
        },
        MovementUpdate::Turn(heading) => {
            client.send(ServerPacket::PlayerMovementResponse(PlayerMovementResponse::new(
                entity.unique_id,
                MovementDestination::direction(false, heading.clone().into()),
                None,
            )));
        },
    }
}

pub(crate) fn update_client(query: Query<(&Client, &GameEntity, &Synchronize)>) {
    for (client, entity, sync) in query.iter() {
        let client: &Client = client;
        let sync: &Synchronize = sync;

        if let Some(movement) = &sync.movement {
            update_movement_for(client, entity, movement);
        }

        if !sync.damage.is_empty() {
            // ...
        }

        if !sync.despawned.is_empty() {
            send_despawns(client, sync);
        }
    }
}

fn send_despawns(client: &Client, sync: &Synchronize) {
    if sync.despawned.len() > 1 {
        client.send(ServerPacket::GroupEntitySpawnStart(GroupEntitySpawnStart::new(
            GroupSpawnType::Despawn,
            sync.despawned.len() as u16,
        )));

        let data = sync
            .despawned
            .iter()
            .map(|id| GroupSpawnDataContent::despawn(*id))
            .collect();

        client.send(ServerPacket::GroupEntitySpawnData(GroupEntitySpawnData::new(data)));

        client.send(ServerPacket::GroupEntitySpawnEnd(GroupEntitySpawnEnd));
    } else {
        let despawned_id = *sync.despawned.get(0).unwrap();
        client.send(ServerPacket::EntityDespawn(EntityDespawn::new(despawned_id)));
    }
}

pub(crate) fn clean_sync(mut query: Query<&mut Synchronize>) {
    for mut sync in query.iter_mut() {
        sync.clear();
    }
}