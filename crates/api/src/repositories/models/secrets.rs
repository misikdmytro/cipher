use interfaces::secrets::{
    ActiveSlot, AwsProviderConfig, BlueGreenStrategyConfig, ProviderConfig, Secret,
    SingleStrategyConfig, StrategyConfig,
};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::Type, PartialEq)]
#[sqlx(type_name = "slot", rename_all = "snake_case")]
pub enum Slot {
    Blue,
    Green,
}

impl From<Slot> for ActiveSlot {
    fn from(slot: Slot) -> Self {
        match slot {
            Slot::Blue => ActiveSlot::Blue,
            Slot::Green => ActiveSlot::Green,
        }
    }
}

#[derive(FromRow, Debug, Clone)]
pub struct SecretRow {
    pub id: Uuid,
    pub cron: String,
    pub strategy_type: String,
    pub provider_type: String,
    pub aws_role_arn: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
    // strategy_single (LEFT JOIN)
    pub single_path: Option<String>,
    // strategy_blue_green (LEFT JOIN)
    pub bg_blue_path: Option<String>,
    pub bg_green_path: Option<String>,
    pub bg_active_slot: Option<Slot>,
}

impl From<SecretRow> for Secret {
    fn from(row: SecretRow) -> Self {
        let strategy = match row.strategy_type.as_str() {
            "single" => StrategyConfig::Single(SingleStrategyConfig {
                path: row
                    .single_path
                    .expect("single_path must be set for single strategy"),
            }),
            "blue_green" => StrategyConfig::BlueGreen(BlueGreenStrategyConfig {
                blue_path: row
                    .bg_blue_path
                    .expect("bg_blue_path must be set for blue_green strategy"),
                green_path: row
                    .bg_green_path
                    .expect("bg_green_path must be set for blue_green strategy"),
                active_slot: row
                    .bg_active_slot
                    .expect("bg_active_slot must be set for blue_green strategy")
                    .into(),
            }),
            other => panic!("unknown strategy_type: {other}"),
        };

        let provider = match row.provider_type.as_str() {
            "aws" => ProviderConfig::Aws(AwsProviderConfig {
                role_arn: row
                    .aws_role_arn
                    .expect("aws_role_arn must be set for aws provider"),
            }),
            other => panic!("unknown provider_type: {other}"),
        };

        Secret {
            id: row.id,
            strategy,
            provider,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
