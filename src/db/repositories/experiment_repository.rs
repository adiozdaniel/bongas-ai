use sqlx::PgPool;

pub struct ExperimentRepository {
    _pool: PgPool,
}

impl ExperimentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { _pool: pool }
    }
}
