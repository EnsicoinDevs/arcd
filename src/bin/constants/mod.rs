pub const MAGIC: u32 = 422021;
pub const VERSION: u32 = 1;
pub const DEFAULT_PORT: &str = "4224";
pub const IP: &str = "127.0.0.1";
pub const DEFAULT_MAX_CONN: &str = "42";
pub const DEFAULT_PROMPT: &str = "7896";
pub const IMPLEMENTATION: &str = "another-rust-coin";

pub type Sha256Result = generic_array::GenericArray<u8, typenum::U32>;
