use sqlx::PgPool;
use crate::error::AppError;

/// Database pool utilities and management
pub struct DatabasePoolManager {
    pool: PgPool,
}

impl DatabasePoolManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get basic pool statistics
    pub async fn get_pool_stats(&self) -> Result<PoolStats, AppError> {
        let size = self.pool.size();
        let idle = self.pool.num_idle() as u32;
        
        Ok(PoolStats {
            size,
            idle,
            active: size - idle,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub size: u32,
    pub idle: u32,
    pub active: u32,
}

/// Get pool statistics for a given pool
pub fn get_pool_stats(pool: &PgPool) -> PoolStats {
    let size = pool.size();
    let idle = pool.num_idle() as u32;
    
    PoolStats {
        size,
        idle,
        active: size - idle,
    }
}
