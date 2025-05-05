// Auto-generated from origin with SHA256 d61ec31277b308093120d09fba81a271596126021c790952740882a82caf91a2.
pub(crate) const CATEGORY_COUNT: u32 = 3;

pub const STATUS_SEVERITY_INFORMATIONAL: u32 = 0x1;
pub const STATUS_SEVERITY_WARNING: u32 = 0x2;
pub const STATUS_SEVERITY_ERROR: u32 = 0x3;
pub const DATABASE_EVENTS_CATEGORY: u16 = 0x00000001;
pub const NETWORK_EVENTS_CATEGORY: u16 = 0x00000002;
pub const UI_EVENTS_CATEGORY: u16 = 0x00000003;
pub const MSG_ERROR: u32 = 0xC0000100;
pub const MSG_WARNING: u32 = 0x80000101;
pub const MSG_INFO: u32 = 0x40000102;
pub const MSG_DEBUG: u32 = 0x40000103;
pub const MSG_TRACE: u32 = 0x40000104;

#[allow(unused_variables)]
pub fn get_category(category: String) -> u16 {

    match category.trim().to_lowercase().as_ref() {
        "\"database events\"" => DATABASE_EVENTS_CATEGORY,
        "\"network events\"" => NETWORK_EVENTS_CATEGORY,
        "\"ui events\"" => UI_EVENTS_CATEGORY,
        _ => 0,
    }

}
