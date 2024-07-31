use crate::comp::exp::Leveled;
use crate::comp::gold::GoldPouch;
use crate::comp::inventory::PlayerInventory;
use crate::comp::net::Client;
use crate::comp::player::{CharacterRace, Player};
use crate::comp::pos::Position;
use crate::comp::GameEntity;
use crate::game::drop::SpawnDrop;
use crate::game::gold::get_gold_ref_id;
use crate::input::PlayerInput;
use bevy_ecs::{prelude::*, world};
use silkroad_definitions::type_id::{
    ObjectClothingPart, ObjectClothingType, ObjectConsumable, ObjectConsumableAmmo, ObjectEquippable, ObjectItem,
    ObjectJewelryType, ObjectRace, ObjectType, ObjectWeaponType,
};
use silkroad_game_base::{Inventory, Item, ItemTypeData, MoveError, Race};
use silkroad_protocol::inventory::{
    InventoryItemData, InventoryOperationError, InventoryOperationRequest, InventoryOperationResponseData,
    InventoryOperationResult,
};
use silkroad_protocol::world::{CharacterEquipItem, CharacterUnequipItem};
use tracing::debug;
use std::any::Any;
use std::cmp::max;
use std::ops::{ControlFlow, Deref};

pub(crate) fn handle_inventory_input(
    mut query: Query<(
        &GameEntity,
        &Client,
        &PlayerInput,
        &Leveled,
        &CharacterRace,
        &mut PlayerInventory,
        &mut GoldPouch,
        &Position,
    )>,
    mut item_spawn: EventWriter<SpawnDrop>,
) {
    for (game_entity, client, input, level, race, mut inventory, mut gold, position) in query.iter_mut() {
        if let Some(ref action) = input.inventory {
            match action.data {
                InventoryOperationRequest::DropGold { amount } => {
                    if handle_player_drop_gold(amount, client, &mut gold, position, &mut item_spawn).is_err() {
                        continue;
                    }
                },
                InventoryOperationRequest::PickupItem { unique_id } => {},
                InventoryOperationRequest::Move { source, target, amount } => {
                    handle_inventory_movement(inventory, source, target, level, race, client, game_entity, amount);
                },
                InventoryOperationRequest::DropItem { .. } => {},
            }
        }
    }
}

fn handle_inventory_movement(mut inventory: Mut<'_, PlayerInventory>, source: u8, target: u8, level: &Leveled, race: &CharacterRace, client: &Client, game_entity: &GameEntity, amount: u16) {
    if let Some(source_item) = inventory.get_item_at(source) {
        match (Inventory::is_equipment_slot(source), Inventory::is_equipment_slot(target)) {
            (false, true) => {
                debug!("false, true");
                // equip item from an item slot
                let fits = item_fits_into_equipment_slot(source_item, target, level, race);
                if fits {
                    // equippable items are always amount 1
                    match inventory.move_item(source, target, 1) {
                        Err(MoveError::Impossible) => {},
                        Err(MoveError::ItemDoesNotExist) => {},
                        Err(MoveError::NotStackable) => {},
                        Ok(amount_moved) => {
                            client.send(InventoryOperationResult::Success(
                                InventoryOperationResponseData::move_item(source, target, amount_moved),
                            ));
                            player_equip_item(&inventory, source, game_entity, client, target);
                        },
                    }
                } else {
                    debug!("Item does not fit the slot {:?}.", source_item);
                    client.send(InventoryOperationResult::Error(InventoryOperationError::Indisposable));
                }
            },
            (true, false) => {
                // unequip item to inventory
                debug!("true, false");
                debug!("moving item from equipped items to inventory");

                if let Some(swapped_in_item) = inventory.get_item_at(target) {
                    // If unequipping to a slot that contains another item 
                    let fits = item_fits_into_equipment_slot(swapped_in_item, source, level, race);
                    if fits {
                        match inventory.move_item(source, target, max(1, amount)) {
                            Err(MoveError::Impossible) => {},
                            Err(MoveError::ItemDoesNotExist) => {},
                            Err(MoveError::NotStackable) => {},
                            Ok(amount_moved) => {
                                client.send(InventoryOperationResult::Success(
                                    InventoryOperationResponseData::move_item(source, target, amount_moved),
                                ));
                                player_unequip_item(&inventory, target, game_entity, client, source);
                                player_equip_item(&inventory, source, game_entity, client, source);
                            }
                        }
                    } else {
                        debug!("Item does not fit the slot.");
                        client.send(InventoryOperationResult::Error(InventoryOperationError::Indisposable));
                    }
                } else {
                    debug!("unequipping item to an empty item slot in inventory");
                        match inventory.move_item(source, target, max(1, amount)) {
                            Err(MoveError::Impossible) => {},
                            Err(MoveError::ItemDoesNotExist) => {},
                            Err(MoveError::NotStackable) => {},
                            Ok(amount_moved) => {
                                client.send(InventoryOperationResult::Success(
                                    InventoryOperationResponseData::move_item(source, target, amount_moved),
                                ));
                                player_unequip_item(&inventory, target, game_entity, client, source);
                            }
                        }
                }
            },
            (false, false) => {
                debug!("false, false");
                // simple inventory movement between slots
                match inventory.move_item(source, target, max(1, amount)) {
                    Err(MoveError::Impossible) => {},
                    Err(MoveError::ItemDoesNotExist) => {},
                    Err(MoveError::NotStackable) => {},
                    Ok(amount_moved) => {
                        client.send(InventoryOperationResult::Success(
                            InventoryOperationResponseData::move_item(source, target, amount_moved),
                        ));
                        if let Some(swapped_in_item) = inventory.get_item_at(target) {
                            debug!("moved {:?} to item slot {:?}", swapped_in_item, source);
                        }
                    },
                }
            }
            (true, true) => {
                debug!("true, true");
                // e.g. swap equipped ring to other ring slot
                let fits = item_fits_into_equipment_slot(source_item, target, level, race);
                if fits {

                } else {
                    debug!("Item does not fit the slot.");
                    client.send(InventoryOperationResult::Error(InventoryOperationError::Indisposable));
                }
            },

        }
    } else {
        client.send(InventoryOperationResult::Error(InventoryOperationError::InvalidTarget));
    }
}

fn player_unequip_item(inventory: &Mut<'_, PlayerInventory>, item_slot_to_be_unequipped: u8, game_entity: &GameEntity, client: &Client, source: u8) {
    if let Some(unequipped_item) = inventory.get_item_at(item_slot_to_be_unequipped) {
        let unequip_msg = CharacterUnequipItem::new(
            game_entity.unique_id,
            source,
            unequipped_item.reference.common.ref_id,
        );
        debug!("unequipping {:?}", unequip_msg);
        client.send(unequip_msg);
    }
}
fn player_equip_item(inventory: &Mut<'_, PlayerInventory>, slot_the_item_gets_equipped_to: u8, game_entity: &GameEntity, client: &Client, slot_of_item_that_got_equipped: u8) {
    if let Some(new_equipment) = inventory.get_item_at(slot_of_item_that_got_equipped) {
        let opt_level = new_equipment.type_data.upgrade_level().unwrap_or(0);
        let equip_msg = CharacterEquipItem::new(
            game_entity.unique_id,
            slot_the_item_gets_equipped_to,
            new_equipment.reference.common.ref_id,
            opt_level,
        );
        debug!("equipping {:?}", equip_msg);
        client.send(equip_msg);
    }
}

fn item_fits_into_equipment_slot(source_item: &Item, target: u8, level: &Leveled, race: &CharacterRace) -> bool {
    let type_id = source_item.reference.common.type_id;
    let object_type = ObjectType::from_type_id(&type_id)
        .expect("Item to equip should have valid object type.");
    let fits = does_object_type_match_slot(target, object_type)
        && source_item
            .reference
            .required_level
            .map(|val| val.get() <= level.current_level())
            .unwrap_or(true)
        && does_object_type_match_race(*race.deref(), object_type);
    // TODO: check if equipment requirement sex matches
    //  check if required masteries matches
    fits
}

fn handle_player_drop_gold(
    amount: u64,
    client: &Client,
    gold: &mut GoldPouch,
    position: &Position,
    item_spawn: &mut EventWriter<SpawnDrop>,
) -> Result<(), ()> {
    if amount > gold.amount() {
        client.send(InventoryOperationResult::Error(InventoryOperationError::NotEnoughGold));
        return Err(());
    }

    if amount == 0 {
        return Err(());
    }

    gold.spend(amount);

    let item_ref = get_gold_ref_id(amount as u32);
    item_spawn.send(SpawnDrop::new(
        Item {
            reference: item_ref,
            variance: None,
            type_data: ItemTypeData::Gold { amount: amount as u32 },
        },
        position.location(),
        None,
    ));

    client.send(InventoryOperationResult::Success(
        InventoryOperationResponseData::DropGold { amount },
    ));

    Ok(())
}


fn weapon_is_onehanded(item: &Item) -> Result<bool, &str> {
    let obj_type = ObjectType::from_type_id(&item.reference.common.type_id).unwrap();
    if let ObjectType::Item(item_type) = obj_type {
        match item_type {
            ObjectItem::Equippable(_) => {
                return Ok(
                    matches!(item_type, ObjectItem::Equippable(ObjectEquippable::Weapon(kind)) 
                        if !matches!(kind, ObjectWeaponType::Glavie | 
                            ObjectWeaponType::Spear | 
                            ObjectWeaponType::Axe | 
                            ObjectWeaponType::Dagger | 
                            ObjectWeaponType::TwoHandSword | 
                            ObjectWeaponType::Harp | 
                            ObjectWeaponType::Staff)),
                )
            },
            _ => return Err("Item is not equippable"),
        };
    }
    Err("Item is not an item")
}

fn does_object_type_match_race(user_race: Race, obj_type: ObjectType) -> bool {
    if let ObjectType::Item(item) = obj_type {
        match item {
            ObjectItem::Equippable(equipment) => match equipment {
                ObjectEquippable::Clothing(kind, _) => {
                    return match kind {
                        ObjectClothingType::Garment | ObjectClothingType::Protector | ObjectClothingType::Armor => {
                            user_race == Race::Chinese
                        },
                        ObjectClothingType::Robe | ObjectClothingType::LightArmor | ObjectClothingType::HeavyArmor => {
                            user_race == Race::European
                        },
                    }
                },
                ObjectEquippable::Shield(race) | ObjectEquippable::Jewelry(race, _) => {
                    return match race {
                        ObjectRace::Chinese => user_race == Race::Chinese,
                        ObjectRace::European => user_race == Race::European,
                    }
                },
                ObjectEquippable::Weapon(kind) => {
                    return match kind {
                        ObjectWeaponType::Sword
                        | ObjectWeaponType::Blade
                        | ObjectWeaponType::Spear
                        | ObjectWeaponType::Glavie
                        | ObjectWeaponType::Bow => user_race == Race::Chinese,
                        ObjectWeaponType::OneHandSword
                        | ObjectWeaponType::TwoHandSword
                        | ObjectWeaponType::Axe
                        | ObjectWeaponType::WarlockStaff
                        | ObjectWeaponType::Staff
                        | ObjectWeaponType::Crossbow
                        | ObjectWeaponType::Dagger
                        | ObjectWeaponType::Harp
                        | ObjectWeaponType::ClericRod => user_race == Race::European,
                        _ => false,
                    }
                },
                _ => {},
            },
            ObjectItem::Consumable(ObjectConsumable::Ammo(kind)) => {
                return match kind {
                    ObjectConsumableAmmo::Arrows => user_race == Race::Chinese,
                    ObjectConsumableAmmo::Bolts => user_race == Race::European,
                }
            },
            _ => {},
        }
    }
    false
}

fn does_object_type_match_slot(slot: u8, obj_type: ObjectType) -> bool {
    if let ObjectType::Item(item) = obj_type {
        match item {
            ObjectItem::Equippable(equipment) => match equipment {
                ObjectEquippable::Clothing(_, part) => {
                    return match part {
                        ObjectClothingPart::Head => slot == 0,
                        ObjectClothingPart::Shoulder => slot == 1,
                        ObjectClothingPart::Body => slot == 2,
                        ObjectClothingPart::Leg => slot == 4,
                        ObjectClothingPart::Arm => slot == 3,
                        ObjectClothingPart::Foot => slot == 5,
                        ObjectClothingPart::Any => false,
                    }
                },
                ObjectEquippable::Shield(_) => {
                    return slot == 7;
                },
                ObjectEquippable::Jewelry(_, kind) => {
                    return match kind {
                        ObjectJewelryType::Earring => slot == 8,
                        ObjectJewelryType::Necklace => slot == 9,
                        ObjectJewelryType::Ring => slot == 11 || slot == 10,
                    }
                },
                ObjectEquippable::Weapon(_) => {
                    return slot == 6;
                },
                _ => {},
            },
            ObjectItem::Consumable(ObjectConsumable::Ammo(_)) => {
                return slot == 7;
            },
            _ => {},
        }
    }
    false
}
