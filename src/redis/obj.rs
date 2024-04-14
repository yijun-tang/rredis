use std::sync::Arc;
use once_cell::sync::Lazy;

/// Our shared "common" objects
pub static CRLF: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("\r\n".to_string()) })
});
pub static OK: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("+OK\r\n".to_string()) })
});
pub static ERR: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("-ERR\r\n".to_string()) })
});
pub static EMPTY_BULK: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("$0\r\n\r\n".to_string()) })
});
pub static C_ZERO: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String(":0\r\n".to_string()) })
});
pub static C_ONE: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String(":1\r\n".to_string()) })
});
pub static NULL_BULK: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("$-1\r\n".to_string()) })
});
pub static NULL_MULTI_BULK: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("*-1\r\n".to_string()) })
});
pub static EMPTY_MULTI_BULK: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("*0\r\n".to_string()) })
});
pub static PONG: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("+PONG\r\n".to_string()) })
});
pub static QUEUED: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("+QUEUED\r\n".to_string()) })
});
pub static WRONG_TYPE_ERR: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("-ERR Operation against a key holding the wrong kind of value\r\n".to_string()) })
});
pub static NO_KEY_ERR: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("-ERR no such key\r\n".to_string()) })
});
pub static SYNTAX_ERR: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("-ERR syntax error\r\n".to_string()) })
});
pub static SAME_OBJECT_ERR: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("-ERR source and destination objects are the same\r\n".to_string()) })
});
pub static OUT_OF_RANGE_ERR: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("-ERR index out of range\r\n".to_string()) })
});
pub static SPACE: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String(" ".to_string()) })
});
pub static COLON: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String(":".to_string()) })
});
pub static PLUS: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("+".to_string()) })
});
pub static SELECT0: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("select 0\r\n".to_string()) })
});
pub static SELECT1: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("select 1\r\n".to_string()) })
});
pub static SELECT2: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("select 2\r\n".to_string()) })
});
pub static SELECT3: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("select 3\r\n".to_string()) })
});
pub static SELECT4: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("select 4\r\n".to_string()) })
});
pub static SELECT5: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("select 5\r\n".to_string()) })
});
pub static SELECT6: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("select 6\r\n".to_string()) })
});
pub static SELECT7: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("select 7\r\n".to_string()) })
});
pub static SELECT8: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("select 8\r\n".to_string()) })
});
pub static SELECT9: Lazy<Arc<RedisObject>> = Lazy::new(|| {
    Arc::new(RedisObject::String { ptr: StringStorageType::String("select 9\r\n".to_string()) })
});


/// Object types
pub enum RedisObject {
    String {
        ptr: StringStorageType,
    },
    List,
    Set,
    ZSet,
    Hash,
}

enum StringStorageType {
    String(String),     // raw string
    Integer(isize),     // encoded as integer
} 

pub fn try_object_sharing(obj: &String) {
    todo!()
}

pub fn try_object_encoding(obj: &String) {
    todo!()
}