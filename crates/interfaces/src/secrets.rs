use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderConfig {
    Aws(AwsProviderConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwsProviderConfig {
    pub role_arn: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveSlot {
    Blue,
    Green,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingleStrategyConfig {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueGreenStrategyConfig {
    pub blue_path: String,
    pub green_path: String,
    pub active_slot: ActiveSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyConfig {
    Single(SingleStrategyConfig),
    BlueGreen(BlueGreenStrategyConfig),
}

impl StrategyConfig {
    pub fn inactive_path(&self) -> Option<&str> {
        match self {
            StrategyConfig::BlueGreen(bg) => match bg.active_slot {
                ActiveSlot::Blue => Some(&bg.green_path),
                ActiveSlot::Green => Some(&bg.blue_path),
            },
            StrategyConfig::Single(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secret {
    pub id: Uuid,
    pub strategy: StrategyConfig,
    pub provider: ProviderConfig,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
}
