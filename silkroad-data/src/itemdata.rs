use crate::common::RefCommon;
use crate::{DataEntry, DataMap, FileError, ParseError};
use num_enum::TryFromPrimitive;
use pk2::Pk2;
use std::num::{NonZeroU16, NonZeroU8};
use std::str::FromStr;

pub fn load_item_map(pk2: &Pk2) -> Result<DataMap<RefItemData>, FileError> {
    DataMap::from(pk2, "/server_dep/silkroad/textdata/ItemData.txt")
}

#[derive(TryFromPrimitive)]
#[repr(u8)]
pub enum RefItemRarity {
    General = 0,
    Blue = 1,
    Seal = 2,
    Set = 3,
    Roc = 6,
    Legend = 8,
}

#[derive(TryFromPrimitive, Copy, Clone, Debug)]
#[repr(u8)]
pub enum RefBiologicalType {
    Female = 0,
    Male = 1,
    Both = 2,
    Pet1 = 3,
    Pet2 = 4,
    Pet3 = 5,
}

impl FromStr for RefBiologicalType {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: u8 = s.parse()?;
        Ok(RefBiologicalType::try_from(value)?)
    }
}

#[derive(Clone, Debug)]
pub struct RefItemData {
    pub common: RefCommon,
    pub price: u64,
    pub max_stack_size: u16,
    pub range: Option<NonZeroU16>,
    pub required_level: Option<NonZeroU8>,
    pub biological_type: RefBiologicalType,
    pub params: [isize; 4],
    pub physical_attack_power_lower: f32, // column 95
    pub physical_attack_power_upper: f32, // column 97
    pub magical_attack_power_lower: f32,  // column 100
    pub magical_attack_power_upper: f32,  // column 102
    pub critical: f32,
    pub physical_reinforce_lower: f32, // column 105
    pub physical_reinforce_upper: f32, // column 107
    pub magical_reinforce_upper: f32,  // column 109
    pub magical_reinforce_lower: f32,  // column 111
    pub attack_rate: f32,              // column 113
}

impl PartialEq for RefItemData {
    fn eq(&self, other: &Self) -> bool {
        self.ref_id() == other.ref_id()
    }
}

impl DataEntry for RefItemData {
    fn ref_id(&self) -> u32 {
        self.common.ref_id
    }

    fn code(&self) -> &str {
        &self.common.id
    }
}

impl FromStr for RefItemData {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let elements = s.split('\t').collect::<Vec<&str>>();
        let common = RefCommon::from_columns(&elements)?;
        let range: u16 = elements.get(94).ok_or(ParseError::MissingColumn(94))?.parse()?;
        let required_level: u8 = elements.get(33).ok_or(ParseError::MissingColumn(33))?.parse()?;

        Ok(Self {
            common,
            price: elements.get(26).ok_or(ParseError::MissingColumn(26))?.parse()?,
            params: [
                elements.get(118).ok_or(ParseError::MissingColumn(118))?.parse()?,
                elements.get(120).ok_or(ParseError::MissingColumn(120))?.parse()?,
                elements.get(122).ok_or(ParseError::MissingColumn(122))?.parse()?,
                elements.get(124).ok_or(ParseError::MissingColumn(124))?.parse()?,
            ],
            range: NonZeroU16::new(range),
            required_level: NonZeroU8::new(required_level),
            biological_type: elements.get(58).ok_or(ParseError::MissingColumn(58))?.parse()?,
            max_stack_size: elements.get(57).ok_or(ParseError::MissingColumn(57))?.parse()?,
            physical_attack_power_lower: elements.get(95).ok_or(ParseError::MissingColumn(95))?.parse()?,
            physical_attack_power_upper: elements.get(97).ok_or(ParseError::MissingColumn(97))?.parse()?,
            magical_attack_power_lower: elements.get(100).ok_or(ParseError::MissingColumn(100))?.parse()?,
            magical_attack_power_upper: elements.get(102).ok_or(ParseError::MissingColumn(102))?.parse()?,

            // for some reason the reinforce columns are always multiplied by 100
            // so we need to divide them by 100 to get the correct value
            physical_reinforce_lower: elements
                .get(105)
                .ok_or(ParseError::MissingColumn(105))?
                .parse::<f32>()?
                / 100.0,
            physical_reinforce_upper: elements
                .get(107)
                .ok_or(ParseError::MissingColumn(107))?
                .parse::<f32>()?
                / 100.0,
            magical_reinforce_lower: elements
                .get(109)
                .ok_or(ParseError::MissingColumn(109))?
                .parse::<f32>()?
                / 100.0,
            magical_reinforce_upper: elements
                .get(111)
                .ok_or(ParseError::MissingColumn(111))?
                .parse::<f32>()?
                / 100.0,

            attack_rate: elements.get(113).ok_or(ParseError::MissingColumn(113))?.parse()?,
            critical: elements.get(116).ok_or(ParseError::MissingColumn(116))?.parse()?,
        })
    }
}
