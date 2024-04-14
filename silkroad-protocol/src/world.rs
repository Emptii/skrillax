use crate::movement::MovementType;
use silkroad_serde::*;

#[derive(Clone, Eq, PartialEq, Copy, Serialize, Deserialize, ByteSize)]
pub enum PvpCape {
    #[silkroad(value = 0)]
    None,
    #[silkroad(value = 1)]
    Red,
    #[silkroad(value = 2)]
    Gray,
    #[silkroad(value = 3)]
    Blue,
    #[silkroad(value = 4)]
    White,
    #[silkroad(value = 5)]
    Yellow,
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Copy, Serialize, ByteSize)]
pub enum AliveState {
    #[silkroad(value = 0)]
    Spawning,
    #[silkroad(value = 1)]
    Alive,
    #[silkroad(value = 2)]
    Dead,
}

#[derive(Clone, Eq, PartialEq, Copy, Serialize, ByteSize)]
pub enum JobType {
    #[silkroad(value = 0)]
    None,
    #[silkroad(value = 1)]
    Trader,
    #[silkroad(value = 2)]
    Thief,
    #[silkroad(value = 3)]
    Hunter,
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Copy, Serialize, ByteSize)]
pub enum PlayerKillState {
    #[silkroad(value = 0xFF)]
    None,
    #[silkroad(value = 1)]
    Purple,
    #[silkroad(value = 2)]
    Red,
}

#[derive(Clone, Eq, PartialEq, Copy, Serialize, ByteSize)]
pub enum ActiveScroll {
    #[silkroad(value = 0)]
    None,
    #[silkroad(value = 1)]
    ReturnScroll,
    #[silkroad(value = 2)]
    JobScroll,
}

#[derive(Clone, Serialize, ByteSize)]
pub enum InteractOptions {
    #[silkroad(value = 0)]
    None,
    #[silkroad(value = 2)]
    Talk { options: Vec<u8> },
}

impl InteractOptions {
    pub fn talk(options: Vec<u8>) -> Self {
        InteractOptions::Talk { options }
    }
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Copy, Serialize, ByteSize)]
pub enum BodyState {
    #[silkroad(value = 0)]
    None,
    #[silkroad(value = 1)]
    Berserk,
    #[silkroad(value = 2)]
    Untouchable,
    #[silkroad(value = 3)]
    GMInvincible,
    #[silkroad(value = 4)]
    GMInvisible,
    #[silkroad(value = 5)]
    Berserk2,
    #[silkroad(value = 6)]
    Stealth,
    #[silkroad(value = 7)]
    Invisible,
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Copy, Serialize, ByteSize)]
pub enum WeatherType {
    #[silkroad(value = 1)]
    Clear,
    #[silkroad(value = 2)]
    Rain,
    #[silkroad(value = 3)]
    Snow,
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Copy, Serialize, ByteSize)]
pub enum ActionState {
    #[silkroad(value = 0)]
    None,
    #[silkroad(value = 2)]
    Walking,
    #[silkroad(value = 3)]
    Running,
    #[silkroad(value = 4)]
    Sitting,
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Copy, Serialize, ByteSize)]
#[silkroad(size = 2)]
pub enum TargetEntityError {
    // FIXME: this is not quite right.
    #[silkroad(value = 0x04)]
    InvalidTarget,
}

#[derive(Clone, Serialize, ByteSize)]
#[silkroad(size = 0)]
pub enum TargetEntityData {
    Monster { unknown: u32, interact_data: Option<u8> },
    NPC { talk_options: Option<InteractOptions> },
}

#[derive(Clone, Serialize, ByteSize)]
pub enum TargetEntityResult {
    #[silkroad(value = 2)]
    Failure { error: TargetEntityError },
    #[silkroad(value = 1)]
    Success {
        unique_id: u32,
        health: Option<u32>,
        entity_data: TargetEntityData,
    },
}

impl TargetEntityResult {
    pub fn failure(error: TargetEntityError) -> Self {
        TargetEntityResult::Failure { error }
    }

    pub fn success_monster(unique_id: u32, health: u32) -> Self {
        TargetEntityResult::Success {
            unique_id,
            health: Some(health),
            entity_data: TargetEntityData::Monster {
                unknown: 0,
                interact_data: Some(5),
            },
        }
    }

    pub fn success_npc(unique_id: u32) -> Self {
        TargetEntityResult::Success {
            unique_id,
            health: None,
            entity_data: TargetEntityData::NPC {
                talk_options: Some(InteractOptions::talk(vec![])),
            },
        }
    }
}

#[derive(Clone, Serialize, ByteSize)]
pub struct EntityState {
    pub alive: AliveState,
    pub unknown1: u8,
    pub action_state: ActionState,
    pub body_state: BodyState,
    pub unknown2: u8,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub berserk_speed: f32,
    pub active_buffs: Vec<ActiveBuffData>,
}

impl EntityState {
    pub fn new(
        alive: AliveState,
        action_state: ActionState,
        body_state: BodyState,
        walk_speed: f32,
        run_speed: f32,
        berserk_speed: f32,
        active_buffs: Vec<ActiveBuffData>,
    ) -> Self {
        EntityState {
            alive,
            unknown1: 0,
            action_state,
            body_state,
            unknown2: 0,
            walk_speed,
            run_speed,
            berserk_speed,
            active_buffs,
        }
    }
}

#[derive(Clone, Serialize, ByteSize)]
pub struct ActiveBuffData {
    pub id: u32,
    pub token: u32,
}

impl ActiveBuffData {
    pub fn new(id: u32, token: u32) -> Self {
        ActiveBuffData { id, token }
    }
}

#[derive(Clone, Serialize, ByteSize)]
pub struct CelestialUpdate {
    pub unique_id: u32,
    pub moon_position: u16,
    pub hour: u8,
    pub minute: u8,
}

impl CelestialUpdate {
    pub fn new(unique_id: u32, moon_position: u16, hour: u8, minute: u8) -> Self {
        CelestialUpdate {
            unique_id,
            moon_position,
            hour,
            minute,
        }
    }
}

#[derive(Clone, Serialize, ByteSize)]
pub struct LunarEventInfo {
    pub unknown_1: u8,
    pub unknown_2: u8,
    pub unknown_3: u32,
    pub unknown_4: u32,
    pub current_achieved: u32,
    pub total: u32,
}

impl LunarEventInfo {
    pub fn new(current_achieved: u32, total: u32) -> Self {
        LunarEventInfo {
            unknown_1: 2,
            unknown_2: 3,
            unknown_3: 1,
            unknown_4: 0x7535,
            current_achieved,
            total,
        }
    }
}

#[derive(Serialize, ByteSize, Copy, Clone)]
pub struct CooldownInfo {
    pub ref_id: u32,
    pub cooldown: u32,
}

#[derive(Serialize, ByteSize, Default, Clone)]
pub struct CharacterFinished {
    pub item_cooldowns: Vec<CooldownInfo>,
    pub skill_cooldowns: Vec<CooldownInfo>,
}

#[derive(Clone, Serialize, ByteSize)]
pub struct WeatherUpdate {
    pub kind: WeatherType,
    pub speed: u8,
}

impl WeatherUpdate {
    pub fn new(kind: WeatherType, speed: u8) -> Self {
        WeatherUpdate { kind, speed }
    }
}

#[derive(Clone, Serialize, ByteSize)]
#[silkroad(size = 2)]
pub enum GameNotification {
    #[silkroad(value = 0xc05)]
    UniqueSpawned { ref_id: u32 },
    #[silkroad(value = 0xc06)]
    UniqueKilled { ref_id: u32, player: String },
}

impl GameNotification {
    pub fn uniquespawned(ref_id: u32) -> Self {
        GameNotification::UniqueSpawned { ref_id }
    }

    pub fn uniquekilled(ref_id: u32, killer: String) -> Self {
        GameNotification::UniqueKilled { ref_id, player: killer }
    }
}

#[derive(Copy, Clone, Serialize, ByteSize)]
pub enum UpdatedState {
    #[silkroad(value = 0)]
    Life(AliveState),
    #[silkroad(value = 1)]
    Movement(MovementType),
    #[silkroad(value = 4)]
    Body(BodyState),
    #[silkroad(value = 7)]
    Pvp(u8),
    #[silkroad(value = 8)]
    Battle(bool),
    #[silkroad(value = 11)]
    Scroll(u8),
}

#[derive(Clone, Copy, Serialize, ByteSize)]
pub struct EntityUpdateState {
    pub unique_id: u32,
    pub update: UpdatedState,
}

impl EntityUpdateState {
    pub fn life(unique_id: u32, new: AliveState) -> Self {
        EntityUpdateState {
            unique_id,
            update: UpdatedState::Life(new),
        }
    }

    pub fn movement(unique_id: u32, new: MovementType) -> Self {
        EntityUpdateState {
            unique_id,
            update: UpdatedState::Movement(new),
        }
    }

    pub fn body(unique_id: u32, new: BodyState) -> Self {
        EntityUpdateState {
            unique_id,
            update: UpdatedState::Body(new),
        }
    }
}

#[derive(Clone, Deserialize, ByteSize)]
pub struct TargetEntity {
    pub unique_id: u32,
}

#[derive(Clone, Serialize, ByteSize)]
pub struct TargetEntityResponse {
    pub result: TargetEntityResult,
}

impl TargetEntityResponse {
    pub fn new(result: TargetEntityResult) -> Self {
        TargetEntityResponse { result }
    }
}

#[derive(Clone, Deserialize, ByteSize)]
pub struct UnTargetEntity {
    pub unique_id: u32,
}

#[derive(Serialize, ByteSize, Copy, Clone)]
pub struct UnTargetEntityResponse {
    pub success: bool,
}

impl UnTargetEntityResponse {
    pub fn new(success: bool) -> Self {
        UnTargetEntityResponse { success }
    }
}

#[derive(Serialize, ByteSize, Copy, Clone)]
#[silkroad(size = 2)]
pub enum EntityBarUpdateSource {
    #[silkroad(value = 0x01)]
    Damage,
    #[silkroad(value = 0x10)]
    Regen,
    #[silkroad(value = 0x80)]
    LevelUp,
}

// Maybe this should be a bitflag?
#[derive(Serialize, ByteSize, Clone)]
pub enum EntityBarUpdates {
    #[silkroad(value = 0)]
    None,
    #[silkroad(value = 1)]
    HP { amount: u32 },
    #[silkroad(value = 2)]
    MP { amount: u32 },
    #[silkroad(value = 3)]
    Both { hp: u32, mp: u32 },
    #[silkroad(value = 4)]
    Status {
        effects: u32,
        #[silkroad(list_type = "none")]
        levels: Vec<u8>,
    },
}

#[derive(Serialize, ByteSize, Clone)]
pub struct EntityBarsUpdate {
    pub unique_id: u32,
    pub source: EntityBarUpdateSource,
    pub updates: EntityBarUpdates,
}

impl EntityBarsUpdate {
    pub fn hp(unique_id: u32, source: EntityBarUpdateSource, hp: u32) -> Self {
        EntityBarsUpdate {
            unique_id,
            source,
            updates: EntityBarUpdates::HP { amount: hp },
        }
    }

    pub fn mp(unique_id: u32, source: EntityBarUpdateSource, mp: u32) -> Self {
        EntityBarsUpdate {
            unique_id,
            source,
            updates: EntityBarUpdates::MP { amount: mp },
        }
    }
}

#[derive(Serialize, ByteSize, Copy, Clone)]
pub enum CharacterPointsUpdate {
    #[silkroad(value = 1)]
    Gold { amount: u64, display: bool },
    #[silkroad(value = 2)]
    SP { amount: u32, display: bool },
    #[silkroad(value = 3)]
    StatPoints(u16),
    #[silkroad(value = 4)]
    Berserk { amount: u8, source: u32 },
}

impl CharacterPointsUpdate {
    pub fn sp(amount: u32) -> CharacterPointsUpdate {
        Self::SP { amount, display: false }
    }
}

#[derive(Clone, Serialize, ByteSize, Debug)]
pub struct CharacterEquipItem {
    pub entity: u32,
    pub slot: u8,
    pub item_ref: u32,
}

#[derive(Clone, Serialize, ByteSize, Debug)]
pub struct CharacterUnequipItem {
    // thank god this guys repo exists, i've spent many hours trying to figure this package out.
    // https://github.com/ferdoran/go-sro-agent-server/blob/6f2b9a9459491254c8eff6707161294ce996d3d9/model/player.go#L252
    pub entity: u32,
    pub slot: u8,
    pub item_ref: u32,
}

impl CharacterUnequipItem {
    pub fn new(entity: u32, slot: u8, item_ref: u32) -> Self {
        CharacterUnequipItem { entity, slot, item_ref }
    }
}

impl CharacterEquipItem {
    pub fn new(entity: u32, slot: u8, item_ref: u32) -> Self {
        CharacterEquipItem { entity, slot, item_ref }
    }
}

#[derive(Serialize, ByteSize, Copy, Clone)]
pub struct PlayerPickupAnimation {
    pub entity: u32,
    pub rotation: u8,
}

#[derive(Serialize, ByteSize, Copy, Clone)]
pub struct LevelUpEffect {
    /// Unique ID of the entity that levelled up
    pub entity: u32,
}

#[derive(Deserialize, Copy, Clone)]
pub struct IncreaseStr;

#[derive(Serialize, ByteSize, Copy, Clone)]
pub enum IncreaseStrResponse {
    #[silkroad(value = 1)]
    Success,
    #[silkroad(value = 2)]
    Error(u16),
}

#[derive(Deserialize, Copy, Clone)]
pub struct IncreaseInt;

#[derive(Serialize, ByteSize, Copy, Clone)]
pub enum IncreaseIntResponse {
    #[silkroad(value = 1)]
    Success,
    #[silkroad(value = 2)]
    Error(u16),
}
