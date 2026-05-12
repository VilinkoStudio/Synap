use std::sync::Arc;

use crate::embedded_redis::EmbeddedRedisHandle;
use crate::redis::RedisRuntime;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    server_name: String,
    redis_runtime: RedisRuntime,
    embedded_redis: Option<EmbeddedRedisHandle>,
    auth: RelayAuth,
}

pub struct AppStateParts {
    pub server_name: String,
    pub redis_runtime: RedisRuntime,
    pub embedded_redis: Option<EmbeddedRedisHandle>,
    pub auth: RelayAuth,
}

#[derive(Clone)]
pub enum RelayAuth {
    Disabled,
    ApiKey(String),
}

impl AppState {
    pub fn from_parts(parts: AppStateParts) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                server_name: parts.server_name,
                redis_runtime: parts.redis_runtime,
                embedded_redis: parts.embedded_redis,
                auth: parts.auth,
            }),
        }
    }

    pub fn server_name(&self) -> &str {
        &self.inner.server_name
    }

    pub fn redis_runtime(&self) -> &RedisRuntime {
        &self.inner.redis_runtime
    }

    pub fn embedded_redis(&self) -> Option<&EmbeddedRedisHandle> {
        self.inner.embedded_redis.as_ref()
    }

    pub fn auth(&self) -> &RelayAuth {
        &self.inner.auth
    }
}
