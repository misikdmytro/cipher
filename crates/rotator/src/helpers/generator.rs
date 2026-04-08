use rand::{RngExt, seq::IndexedRandom};

pub trait SecretGenerator: Send + Sync {
    fn generate(&self) -> String;
}

pub struct RandomHexGenerator {
    byte_length: usize,
}

impl RandomHexGenerator {
    pub fn new(byte_length: usize) -> Self {
        Self { byte_length }
    }
}

impl SecretGenerator for RandomHexGenerator {
    fn generate(&self) -> String {
        let mut bytes = vec![0u8; self.byte_length];
        rand::rng().fill(&mut bytes[..]);
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

const CHARSET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[]{}|;:,.<>?";

pub struct RandomPasswordGenerator {
    length: usize,
}

impl RandomPasswordGenerator {
    pub fn new(length: usize) -> Self {
        Self { length }
    }
}

impl SecretGenerator for RandomPasswordGenerator {
    fn generate(&self) -> String {
        let mut rng = rand::rng();
        (0..self.length)
            .map(|_| *CHARSET.choose(&mut rng).unwrap() as char)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_correct_hex_length() {
        let generator = RandomHexGenerator::new(32);
        let secret = generator.generate();
        assert_eq!(secret.len(), 64, "32 bytes should produce 64 hex chars");
    }

    #[test]
    fn generates_valid_hex() {
        let generator = RandomHexGenerator::new(16);
        let secret = generator.generate();
        assert!(
            secret.chars().all(|c| c.is_ascii_hexdigit()),
            "output should contain only hex characters, got: {secret}"
        );
    }

    #[test]
    fn generates_unique_values() {
        let generator = RandomHexGenerator::new(32);
        let a = generator.generate();
        let b = generator.generate();
        assert_ne!(
            a, b,
            "two consecutive calls should produce different values"
        );
    }

    #[test]
    fn password_generates_correct_length() {
        let generator = RandomPasswordGenerator::new(48);
        let secret = generator.generate();
        assert_eq!(secret.len(), 48);
    }

    #[test]
    fn password_contains_special_characters() {
        let generator = RandomPasswordGenerator::new(200);
        let secret = generator.generate();
        assert!(
            secret
                .chars()
                .any(|c| "!@#$%^&*()-_=+[]{}|;:,.<>?".contains(c)),
            "password should contain at least one special character, got: {secret}"
        );
    }

    #[test]
    fn password_generates_unique_values() {
        let generator = RandomPasswordGenerator::new(48);
        let a = generator.generate();
        let b = generator.generate();
        assert_ne!(
            a, b,
            "two consecutive calls should produce different values"
        );
    }
}
